use argh::FromArgs;
use futures::join;
use hashindex_rs::hashindex_rs;
use smol::channel;

#[derive(FromArgs, PartialEq, Debug)]
#[argh(
    description = "\n HASHINDEX-RS is a tool to hash all the contained files in the provided path.
\nFeatures:
\n - It sends to stdout the results in comma separated as [label], [hash(s)], [size (kbs)], [path]
\n - It runs a number of tasks equal to the number of cores of the system
\n - It ignores links
\n - See the optional arguments for possible customisations
\n`
\nWarning: The hash created are not cryptographically strong It calculates a 64 bit hash for each item.
\nWarning: This tool will not follow links.
\nWarning: The order of the hash map presented will not necesarily be deterministic"
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

    /// list of hash algorithms to use. default algorithm `xxh3`. Order matters choose from xxh64, xxh3.
    /// use comma separater list such as --hash-list xxh64,xxh3 or --hash-list "xxh64, xxh3"
    #[argh(option, short = 'h')]
    hash_list: Option<String>,

    /// number of jobs to use to compute hashes. defaults to the number of cores
    #[argh(option, short = 'j')]
    jobs: Option<usize>,

    // TODO: Make the version argument overrides even the positional arguments
    /// prints the version of the application and exits. It will ignore any other parameter
    #[argh(switch, short = 'v')]
    version: bool,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Arguments = argh::from_env();

    if args.version {
        println!(
            "{} v{}",
            env!("CARGO_PKG_NAME").to_ascii_uppercase(),
            env!("CARGO_PKG_VERSION")
        );
        println!("Authors: {}", env!("CARGO_PKG_AUTHORS"));
        println!("Repository: {}", env!("CARGO_PKG_REPOSITORY"));
        std::process::exit(0);
    }
    let delimiter = match args.delimiter {
        Some(delimiter) => delimiter,
        None => ",".into(),
    };

    let hash_algorithms = match args.hash_list {
        Some(hl) => {
            let (valid_hash, invalid_hash) = hashindex_rs::check_hash(&hl);
            if !invalid_hash.is_empty() {
                eprintln!("Provided unimplemented hash algorithms: {:?}", invalid_hash);
                eprintln!(
                    "Implemented hash algorithms: {:?}",
                    hashindex_rs::hash_variants()
                );
                std::process::exit(1);
            }
            valid_hash
        }
        None => vec![hashindex_rs::default_hash()],
    };

    let number_of_workers = match args.jobs {
        Some(jobs) => jobs,
        None => num_cpus::get(),
    };

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
