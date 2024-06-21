use clap::Parser;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Conf {
    #[clap(long)]
    pub bind: String,
}

impl Conf {
    pub fn from_cmd_line() -> Conf {
        Conf::parse()
    }
}