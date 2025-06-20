mod hasher_wrapper;

pub mod hashindex_rs {

    /// I want to
    /// Re-exported to make it easier to validate available hash algorithms
    pub use crate::hasher_wrapper::check_hash;
    pub use crate::hasher_wrapper::default_hash;
    pub use crate::hasher_wrapper::variants as hash_variants;

    use futures::io::AsyncReadExt;
    use smol::{
        channel,
        fs::{self, File},
        stream::StreamExt,
    };
    use std::{
        io::{Error, ErrorKind},
        path::PathBuf,
    };

    use crate::hasher_wrapper::{HasherWrapper, new_xxh3, new_xxh64};

    // TODO: Remove this duplicity with the module hasher_wrapper
    // It implies the same information as we do with the mentioned module
    // I implemented it when I was experimenting with more than one hash algorithm
    // possibly in the commit: e8334fab206ef3469ada367dae0b88b89f635341
    #[derive(Clone)]
    enum HashAlgorithm {
        Xxh64,
        Xxh3,
    }
    impl HashAlgorithm {
        #[allow(dead_code)]
        fn from_str(s: &str) -> Option<Self> {
            match s.to_lowercase().as_str() {
                "xxh64" => Some(HashAlgorithm::Xxh64),
                "xxh3" => Some(HashAlgorithm::Xxh3),
                _ => None,
            }
        }
        fn from_string(s: String) -> Option<Self> {
            match s.to_lowercase().as_str() {
                "xxh64" => Some(HashAlgorithm::Xxh64),
                "xxh3" => Some(HashAlgorithm::Xxh3),
                _ => None,
            }
        }
    }

    /// Initiates a path explorer on the given path and sends the found files to
    /// the workers using the provided channel.
    ///
    /// Returns an error if the path does not exist
    pub async fn explore_path(
        path: &str,
        sender: channel::Sender<PathBuf>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let path = PathBuf::from(path);
        if !path.exists() {
            return Err(Error::new(ErrorKind::NotFound, "Path not found").into());
        }
        explore_folder_inner_stacked(&PathBuf::from(path), sender).await?;

        Ok(())
    }

    /// Inner function that does the actual work of exploring the
    /// folder and sending the file path to the workers over a channel.
    async fn explore_folder_inner_stacked(
        path: &PathBuf,
        sender: channel::Sender<PathBuf>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut dir_stack = vec![path.clone()];
        while let Some(dir) = dir_stack.pop() {
            if let Ok(mut dir_entries) = fs::read_dir(dir).await {
                while let Some(entry) = dir_entries.try_next().await? {
                    let path = entry.path();
                    if path.is_dir() {
                        dir_stack.push(path);
                    } else if path.is_file() {
                        if sender.is_closed() {}
                        let _ = sender.send_blocking(path);
                    }
                    // We are only interested in exploring files and directories so we ignore links
                }
            }
        }

        Ok(())
    }

    /// Runs workers to compute the hash and print to the stdout the result.
    ///
    /// # Errors
    ///
    /// This function will return an error if ...
    pub async fn run_workers(
        label: String,
        delimiter: String,
        hash_algorithms: Vec<String>,
        receive: channel::Receiver<PathBuf>,
        number_of_workers: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let number_of_workers = number_of_workers.max(1);

        let mut workers = Vec::with_capacity(number_of_workers);

        let hash_algorithms: Vec<HashAlgorithm> = hash_algorithms
            .into_iter()
            .filter_map(HashAlgorithm::from_string)
            .collect();

        for _ in 0..number_of_workers {
            let task_receiver = receive.clone();
            let task_label = label.to_string();
            let task_delimiter = delimiter.clone();
            let task_hash_algorithms = hash_algorithms.clone();
            workers.push(smol::spawn(async move {
                work_print(
                    task_label,
                    task_delimiter,
                    task_hash_algorithms,
                    task_receiver,
                )
                .await;
            }));
        }

        for worker in workers {
            worker.await;
        }

        Ok(())
    }

    /// Worker function to print the properties selected of a file that is received via a channel
    async fn work_print(
        label: String,
        delimiter: String,
        task_hash_algorithms: Vec<HashAlgorithm>,
        task_receiver: channel::Receiver<PathBuf>,
    ) {
        loop {
            if let Ok(path_buf) = task_receiver.recv().await {
                if !path_buf.is_file() {
                    continue;
                } else {
                    let hash = match calc_hashes(&path_buf, &task_hash_algorithms).await {
                        Ok(hash) => hash.join(&delimiter),
                        Err(err) => {
                            eprintln!("Failed to calculate hash for {path_buf:?}: {err}");
                            continue;
                        }
                    };
                    let size = match path_buf.metadata() {
                        Ok(md) => md.len(),
                        Err(err) => {
                            eprintln!("Failed to obtain size for {path_buf:?}: {err}");
                            continue;
                        }
                    };
                    println!("{label:}{delimiter}{hash:}{delimiter}{size:}{delimiter}{path_buf:?}");
                }
            };
            if task_receiver.is_closed() {
                break;
            }
        }
    }

    /// Computes the list of hashes provided using the same stream saving expensive access time
    async fn calc_hashes(
        path: &PathBuf,
        task_hash_algorithms: &Vec<HashAlgorithm>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut file = File::open(path).await?;
        let mut hashers = vec![];
        for algorithm in task_hash_algorithms {
            let new_hasher = match algorithm {
                HashAlgorithm::Xxh64 => HasherWrapper::Xxh64(new_xxh64()),
                HashAlgorithm::Xxh3 => HasherWrapper::Xxh3(new_xxh3()),
            };
            hashers.push(new_hasher);
        }

        let mut buffer: [u8; 8192] = [0; 8192];
        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break; // End of file
            }

            hashers
                .iter_mut()
                .for_each(|hasher| hasher.update(&buffer[..bytes_read]));
        }

        let hashes: Vec<String> = hashers.iter().map(|hash| hash.finish()).collect();
        Ok(hashes)
    }
}

#[cfg(test)]
mod tests {

    use std::fs;

    use crate::hashindex_rs;
    use futures::join;
    use smol::channel;
    use tempfile::NamedTempFile;

    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    fn prepare_channel<T>() -> (channel::Sender<T>, channel::Receiver<T>) {
        channel::bounded(1)
    }

    #[test]
    fn bad_path() {
        let path = "invalid Path which will not resolve in any real path";
        let delimiter = ",";

        let (sender, receiver) = prepare_channel();
        smol::block_on(async {
            let (_, explore_result) = join!(
                hashindex_rs::run_workers(
                    "label".into(),
                    delimiter.into(),
                    hashindex_rs::hash_variants(),
                    receiver,
                    1
                ),
                hashindex_rs::explore_path(&path, sender),
            );
            assert!(explore_result.is_err());
        });
    }

    #[test]
    fn valid_path() {
        let (_temp_file, temp_path) = make_temp_file();

        let path = temp_path.to_str().unwrap();
        let delimiter = ",";

        let (sender, receiver) = prepare_channel();
        smol::block_on(async {
            let (_, explore_result) = join!(
                hashindex_rs::run_workers(
                    "label".into(),
                    delimiter.into(),
                    hashindex_rs::hash_variants(),
                    receiver,
                    1
                ),
                hashindex_rs::explore_path(&path, sender),
            );
            assert!(explore_result.is_ok());
        });
    }

    #[cfg(unix)]
    #[test]
    fn no_path_permissions() {
        // Create a temporary file with random content
        let (_named_temp_file, temp_path) = make_temp_file();

        // Change file permissions to make it unreadable
        let mut permissions = fs::metadata(&temp_path).unwrap().permissions();
        permissions.set_mode(0o000); // No permissions
        fs::set_permissions(&temp_path, permissions).unwrap();
        let delimiter = ",";
        let (sender, receiver) = prepare_channel();

        smol::block_on(async {
            let (_, explore_result) = join!(
                hashindex_rs::run_workers(
                    "label".into(),
                    delimiter.into(),
                    hashindex_rs::hash_variants(),
                    receiver,
                    1
                ),
                hashindex_rs::explore_path(&temp_path.to_str().unwrap(), sender),
            );
            assert!(explore_result.is_ok()); // The program should not panic
        });

        // Cleanup: Restore permissions to allow deletion
        let mut permissions = fs::metadata(&temp_path).unwrap().permissions();
        permissions.set_mode(0o644); // Read/write for owner, read for others
        fs::set_permissions(&temp_path, permissions).unwrap();
    }

    fn make_temp_file() -> (NamedTempFile, std::path::PathBuf) {
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path().to_path_buf();
        fs::write(&temp_path, "random content").unwrap();
        (temp_file, temp_path)
    }
}
