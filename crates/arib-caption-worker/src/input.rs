use std::{fs::File, io, path::Path};

#[cfg(not(test))]
pub(crate) fn open_input(path: &Path) -> io::Result<File> {
    File::open(path)
}

#[cfg(test)]
pub(crate) use measured::{measure_reads, open_input};

#[cfg(test)]
mod measured {
    use super::*;
    use std::{
        cell::RefCell,
        io::{Read, Seek, SeekFrom},
        path::PathBuf,
    };

    #[derive(Default, Debug)]
    pub(crate) struct Reads {
        pub opens: usize,
        pub bytes: u64,
    }
    thread_local! {
        static MEASUREMENT: RefCell<Option<(PathBuf, Reads)>> = const { RefCell::new(None) };
    }
    pub(crate) fn measure_reads<T>(path: &Path, run: impl FnOnce() -> T) -> (T, Reads) {
        struct Reset;
        impl Drop for Reset {
            fn drop(&mut self) {
                MEASUREMENT.with(|state| *state.borrow_mut() = None);
            }
        }
        MEASUREMENT.with(|state| {
            assert!(state.borrow().is_none(), "nested input measurement");
            *state.borrow_mut() = Some((path.to_owned(), Reads::default()));
        });
        let _reset = Reset;
        let result = run();
        let reads = MEASUREMENT.with(|state| state.borrow_mut().take().unwrap().1);
        (result, reads)
    }
    pub(crate) struct InputFile {
        file: File,
        measured: bool,
    }
    pub(crate) fn open_input(path: &Path) -> io::Result<InputFile> {
        let file = File::open(path)?;
        let measured = MEASUREMENT.with(|state| {
            let mut state = state.borrow_mut();
            if let Some((target, reads)) = state.as_mut()
                && target == path
            {
                reads.opens += 1;
                true
            } else {
                false
            }
        });
        Ok(InputFile { file, measured })
    }
    impl InputFile {
        pub(crate) fn metadata(&self) -> io::Result<std::fs::Metadata> {
            self.file.metadata()
        }
    }
    impl Read for InputFile {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let count = self.file.read(buffer)?;
            if self.measured {
                MEASUREMENT.with(|state| {
                    if let Some((_, reads)) = state.borrow_mut().as_mut() {
                        reads.bytes += count as u64;
                    }
                });
            }
            Ok(count)
        }
    }
    impl Seek for InputFile {
        fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
            self.file.seek(position)
        }
    }
}
