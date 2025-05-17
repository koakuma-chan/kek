use crate::file_processor::CategoryData;
use rustix::fd::{AsFd, BorrowedFd}; 
use rustix::fs::{open, sendfile, Mode, OFlags};
use rustix::io as rustix_io;
use rustix::stdio;
use std::fmt::Display;
use std::io::{self, BufWriter, Write};

/// A wrapper around `BorrowedFd` to implement `std::io::Write`.
/// This allows `rustix` file descriptors to be used with `std::io::BufWriter`
/// and other `std::io` utilities.
struct FdWriter<'a> {
    fd: BorrowedFd<'a>,
}

impl<'a> io::Write for FdWriter<'a> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        match rustix_io::write(self.fd, buf) {
            Ok(0) if !buf.is_empty() => {
                Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "FdWriter: rustix::io::write returned 0 bytes written, but buffer was not empty.",
                ))
            }
            Ok(n) => Ok(n),
            Err(e) if e == rustix_io::Errno::INTR => {
                Err(io::Error::new(io::ErrorKind::Interrupted, e))
            }
            Err(e) => {
                Err(io::Error::from(e))
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Helper to write a string slice followed by a newline to an `io::Write` implementor.
fn write_str_line_to_writer(writer: &mut impl Write, text: &str) -> io::Result<()> {
    writer.write_all(text.as_bytes())?;
    writer.write_all(b"\n")
}

/// Helper to write a Display-able type followed by a newline to an `io::Write` implementor.
fn write_display_line_to_writer<T: Display>(writer: &mut impl Write, item: T) -> io::Result<()> {
    writeln!(writer, "{}", item)
}

/// Writes the processed category data and optional task arguments to stdout.
/// Metadata (XML-like tags, descriptions, paths, task arguments) is written using a `BufWriter`
/// wrapping stdout for buffered I/O.
/// File content is streamed directly using `sendfile` after flushing the buffer.
///
/// Business Logic Constraint: Output is pseudo-XML, not strictly valid XML. No escaping is performed.
/// Business Logic Constraint: File content is written raw via `sendfile`.
/// Business Logic Constraint: If `task_args` is `Some`, it will be printed as `<task>{args}</task>`
/// at the end of the output, even if `categories_data` is empty.
pub fn write_output(
    categories_data: &[CategoryData],
    task_args: Option<String>,
) -> io::Result<()> {
    // If there's no category data and no task arguments, there's nothing to do.
    if categories_data.is_empty() && task_args.is_none() {
        return Ok(());
    }

    // Obtain an OwnedFd for stdout from rustix, then immediately get a BorrowedFd.
    // The BorrowedFd's lifetime is tied to the scope of this function call where stdout_owned_fd exists.
    let stdout_owned_fd = stdio::stdout();
    let stdout_borrowed_fd = stdout_owned_fd.as_fd();

    let fd_writer_for_stdout = FdWriter {
        fd: stdout_borrowed_fd,
    };
    let mut buffered_stdout = BufWriter::new(fd_writer_for_stdout);

    for category_data in categories_data {
        write_str_line_to_writer(&mut buffered_stdout, "<category>")?;

        write_str_line_to_writer(&mut buffered_stdout, "<description>")?;
        write_display_line_to_writer(&mut buffered_stdout, &category_data.description_text)?;
        write_str_line_to_writer(&mut buffered_stdout, "</description>")?;

        write_str_line_to_writer(&mut buffered_stdout, "<files>")?;

        for file_data in &category_data.files {
            write_str_line_to_writer(&mut buffered_stdout, "<file>")?;

            write_str_line_to_writer(&mut buffered_stdout, "<path>")?;
            write_display_line_to_writer(&mut buffered_stdout, file_data.relative_path.display())?;
            write_str_line_to_writer(&mut buffered_stdout, "</path>")?;

            write_str_line_to_writer(&mut buffered_stdout, "<content>")?;
            buffered_stdout.flush()?; // Flush metadata before sendfile

            let file_to_send_owned_fd =
                open(&file_data.absolute_path, OFlags::RDONLY, Mode::empty()).map_err(|e| {
                    io::Error::new(
                        io::ErrorKind::Other,
                        format!(
                            "Failed to open file {:?} for sendfile: {}",
                            file_data.absolute_path, e
                        ),
                    )
                })?;

            let file_size: usize = file_data.size.try_into().unwrap();

            if file_size > 0 {
                let mut sent_total = 0usize;
                let file_to_send_borrowed_fd = file_to_send_owned_fd.as_fd();
                while sent_total < file_size {
                    let remaining_to_send = file_size - sent_total;
                    match sendfile(
                        stdout_borrowed_fd,
                        file_to_send_borrowed_fd,
                        None,
                        remaining_to_send,
                    ) {
                        Ok(0) => {
                            return Err(io::Error::new(
                                io::ErrorKind::WriteZero,
                                format!(
                                    "sendfile sent 0 bytes for {:?} before completion (sent {} of {}). File may have been truncated or output pipe closed.",
                                    file_data.absolute_path, sent_total, file_size
                                ),
                            ));
                        }
                        Ok(bytes_sent_this_call) => {
                            sent_total += bytes_sent_this_call;
                        }
                        Err(e) if e == rustix_io::Errno::INTR => continue,
                        Err(e) => {
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                format!("sendfile failed for {:?}: {}", file_data.absolute_path, e),
                            ));
                        }
                    }
                }
            }
            // Write a newline after the file content; this goes through the buffer.
            buffered_stdout.write_all(b"\n")?;
            write_str_line_to_writer(&mut buffered_stdout, "</content>")?;
            write_str_line_to_writer(&mut buffered_stdout, "</file>")?;
        }
        write_str_line_to_writer(&mut buffered_stdout, "</files>")?;
        write_str_line_to_writer(&mut buffered_stdout, "</category>")?;
    }

    // After all categories and files, write the task arguments if present.
    // Business Logic Constraint: If command line arguments were provided to the program
    // (after the program name), they are joined by spaces and printed here,
    // wrapped in <task> tags. This occurs even if the joined string is empty
    // (e.g., if the only argument was an empty string).
    if let Some(joined_args) = task_args {
        write_str_line_to_writer(&mut buffered_stdout, &format!("<task>{}</task>", joined_args))?;
    }

    buffered_stdout.flush()?; // Ensure all buffered data, including task args, is written.
    Ok(())
}
