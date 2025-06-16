use argh::FromArgs;
use futures::join;
use hashindex_rs::hashindex_rs;
use smol::channel;

#[derive(FromArgs, PartialEq, Debug)]
#[argh(
    description = "\n hashindex is a tool to hash all the contained filesin a path, add an identifier for the files in the given folder.
\nFeatures:
\n - It sends to stdout the results in comma separated as [label], [hash], [path]
\n - It runs a number of tasks equal to the number of cores of the system
\n - It ignores links
\n`
\nWarning: The hash created are not cryptographically strong It calculates a 64 bit hash for each item.
\nWarning: This tool will not follow links."
)]
struct Arguments {
    /// the base path to explore
    #[argh(positional)]
    base_path: String,

    /// the label for the dataset is mandatory
    #[argh(positional)]
    label: String,

    /// the field delitimer. It will accept a string
    #[argh(option, short = 'd')]
    delimiter: Option<String>,
    // /// print results to std out
    // #[argh(switch, short = 's')]
    // std_out: bool,
    // TODO: Add an option to save to a file or std. For now just stdout
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Arguments = argh::from_env();

    let delimiter = match args.delimiter {
        Some(delimiter) => delimiter,
        None => ",".into(),
    };

    // TODO: Add a parameter to choose the hash algorithm
    let hash_algorithms = vec!["xxh64", "xxh3"];

    // TODO: Add a parameter to choose the number of workers
    let number_of_workers = num_cpus::get();

    // TODO: List the file size as another field as a number of bytes

    let (sender, receive) = channel::bounded(number_of_workers);

    smol::block_on(async {
        let (_workers, _explorer) = join!(
            hashindex_rs::run_workers(
                args.label.into(),
                delimiter,
                hash_algorithms,
                receive,
                number_of_workers
            ),
            hashindex_rs::explore_path(&args.base_path, sender),
        );

        if let Err(e) = _explorer {
            eprintln!("Error exploring path: {}", e);
        }
        if let Err(e) = _workers {
            eprintln!("Error running workers: {}", e);
        }
    });

    Ok(())
}
