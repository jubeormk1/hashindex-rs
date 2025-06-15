pub mod hashindex_rs {

    use futures::io::AsyncReadExt;
    use smol::{
        channel,
        fs::{self, File},
        stream::StreamExt,
    };
    use std::{
        hash::Hasher,
        io::{Error, ErrorKind},
        path::PathBuf,
    };
    use twox_hash::XxHash64;

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
    /// This function will return an error if .
    pub async fn run_workers(
        label: String,
        delimiter: String,
        receive: channel::Receiver<PathBuf>,
        number_of_workers: usize,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut workers = Vec::with_capacity(number_of_workers);
        for _ in 0..number_of_workers {
            let task_receiver = receive.clone();
            let task_label = label.to_string();
            let task_delimiter = delimiter.clone();
            workers.push(smol::spawn(async move {
                work_print(task_label, task_delimiter, task_receiver).await;
            }));
        }

        for worker in workers {
            worker.await;
        }

        Ok(())
    }

    async fn work_print(
        label: String,
        delimiter: String,
        task_receiver: channel::Receiver<PathBuf>,
    ) {
        loop {
            if let Ok(path_buf) = task_receiver.recv().await {
                if !path_buf.is_file() {
                    continue;
                } else {
                    let hash = calc_hash(&path_buf).await.unwrap();

                    println!("{label:} {delimiter} {hash:} {delimiter} {path_buf:?}");
                }
            };
            if task_receiver.is_closed() {
                break;
            }
        }
    }

    async fn calc_hash(path: &PathBuf) -> Result<u64, Box<dyn std::error::Error>> {
        let mut file = File::open(path).await?;
        let mut hasher = XxHash64::default();
        let mut buffer: [u8; 8192] = [0; 8192]; // Read in 8KB chunks

        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break; // End of file
            }
            hasher.write(&buffer[..bytes_read]);
        }

        Ok(hasher.finish())
    }
}

#[cfg(test)]
mod tests {

    use crate::hashindex_rs;
    use futures::join;
    use smol::channel;

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
                hashindex_rs::run_workers("label".into(), delimiter.into(), receiver, 1),
                hashindex_rs::explore_path(&path, sender),
            );
            assert!(explore_result.is_err());
        });
    }

    #[test]
    fn valid_path() {
        let path = "./";
        let (sender, receiver) = prepare_channel();
        let delimiter = ",";

        smol::block_on(async {
            let (_, explore_result) = join!(
                hashindex_rs::run_workers("label".into(), delimiter.into(), receiver, 1),
                hashindex_rs::explore_path(&path, sender),
            );
            assert!(explore_result.is_ok());
        });
    }

    #[test]
    fn no_path_permissions() {
        todo!(
            "Not implemented: What happens if the path is good but the user does not have enough permissions to read it"
        );

        // let path_string = "/root/";
        // future::block_on(async {
        //     let explore_result = hashindex_rs::explore_path(path_string).await;
        //     assert!(explore_result.is_ok());
        // });
    }
}
