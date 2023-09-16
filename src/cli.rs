
    use std::error::Error;    
    use clap::Parser;
    use std::io::ErrorKind;

    /// Simple program to greet a person
    #[derive(Parser, Debug)]
    #[command(author, version, about, long_about = None)]
    pub struct Args {
        /// Name of first network interface
        #[arg(long)]
        pub iface1: String,
        /// Name of second network interface
        #[arg(long)]
        pub iface2: String,
        
        /// Log severity
        #[arg(long, default_value_t = String::from("debug"))]
        pub log_level: String,
    }
    
pub fn handling_args()->  Result<(), Box<dyn Error>>{
        let args = Args::parse();
        println!("{args:?}");

      
        Ok(())
}




