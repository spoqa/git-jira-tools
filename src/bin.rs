#[macro_use]
extern crate log;

extern crate docopt;
extern crate hyper;
extern crate regex;
extern crate rustc_serialize;
extern crate url;

use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::io;
use std::io::{BufRead, Write};
use std::process::Command;
use std::str::FromStr;

use docopt::Docopt;
use hyper::Client;
use hyper::header::{Authorization, Basic};
use regex::Regex;
use rustc_serialize::json::Json;
use url::{Url, UrlParser};


static USAGE: &'static str = r#"
Usage: git-jira branch
       git-jira --help

Options:
  -h, --help    Show this message.
"#;

#[derive(RustcDecodable, Debug)]
struct Args {
    cmd_branch: bool,
}

struct Config {
    base_url: Url,
    credential: hyper::header::Basic,
}

impl Config {
    fn from_git_config() -> Result<Config, Box<Error>> {
        let base_url = try!(read_config_value("com.spoqa.jira.url", "JIRA URL"));
        let base_url = Url::parse(&base_url).unwrap();
        let cred = try!(read_credential());
        Ok(Config {
            base_url: base_url,
            credential: cred,
        })
    }
}

fn read_value(prompt: &str) -> Result<String, Box<Error>> {
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();
    print!("{}: ", prompt);
    try!(stdout.flush());
    let mut line = String::new();
    try!(stdin.read_line(&mut line));
    Ok(line.trim().to_owned())
}

fn load_config_value(key: &str) -> Result<Option<String>, Box<Error>> {
    let output = try!(Command::new("git").arg("config").arg(key).output());
    if !output.status.success() {
        Ok(None)
    } else {
        let value = try!(std::str::from_utf8(&output.stdout)).trim();
        if value.is_empty() {
            Ok(None)
        } else {
            Ok(Some(value.to_owned()))
        }
    }
}

fn save_config_value(key: &str, value: &str) -> Result<(), Box<Error>> {
    try!(Command::new("git").arg("config").arg("--global").arg(key).arg(value).status());
    Ok(())
}

fn read_config_value(key: &str, prompt: &str) -> Result<String, Box<Error>> {
    match try!(load_config_value(key)) {
        Some(value) => Ok(value),
        None => {
            let value = try!(read_value(prompt));
            try!(save_config_value(key, &value));
            Ok(value)
        }
    }
}

fn read_credential() -> Result<hyper::header::Basic, Box<Error>> {
    let key = "com.spoqa.jira.credential";
    if let Some(value) = try!(load_config_value(key)) {
        if let Ok(value) = FromStr::from_str(&value) {
            return Ok(value)
        }
    }
    let username = try!(read_value("Username"));
    let password = try!(read_value("Password"));
    let cred = hyper::header::Basic {
        username: username,
        password: Some(password),
    };
    struct SchemeExporter<'a, S: 'a + hyper::header::Scheme>(&'a S);
    impl<'a, S: 'a> fmt::Display for SchemeExporter<'a, S> where S: hyper::header::Scheme {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.0.fmt_scheme(f) }
    }
    try!(save_config_value(key, &format!("{}", SchemeExporter(&cred))));
    Ok(cred)
}

fn main() {
    let args: Args = Docopt::new(USAGE).and_then(|d| d.decode()).unwrap_or_else(|e| e.exit());
    let config = Config::from_git_config().unwrap();
    if args.cmd_branch {
        branch(config);
    }
}

fn branch(config: Config) {
    let key_pattern = Regex::new(r"[A-Z]+-\d+").unwrap();

    let output = Command::new("git").arg("branch").arg("--list").arg("--no-column")
        .output()
        .unwrap_or_else(|e| { panic!("failed to execute process: {}", e) });
    if !output.status.success() {
        io::stderr().write_all(&output.stderr[..]).unwrap();
        return;
    }

    let branches: Vec<_> = io::BufReader::new(&output.stdout[..]).lines().map(Result::unwrap).collect();
    let keys: Vec<_> = branches.iter().map(|b| key_pattern.captures(&b).and_then(|caps| caps.at(0))).collect();
    let mut url = UrlParser::new().base_url(&config.base_url).parse("/rest/api/2/search").unwrap();
    url.set_query_from_pairs(vec![
        ("jql", &format!("key in ({})", keys.iter().filter_map(|&e| e).collect::<Vec<_>>().connect(","))[..]),
        ("fields", "summary"),
    ].into_iter());
    debug!("URL: {}", url);
    let mut client = Client::new();
    let mut res = client.get(url)
        .header(Authorization(config.credential))
        .send().unwrap();
    let json = Json::from_reader(&mut res).unwrap();
    let issues = json["issues"].as_array().unwrap();
    let summary_map: BTreeMap<_, _> = issues.iter().map(|e| (e["key"].as_string().unwrap(), e["fields"]["summary"].as_string().unwrap())).collect();
    debug!("response JSON:\n{}", json.pretty());
    for (b, k) in branches.iter().zip(&keys) {
        let summary = match *k {
            Some(k) => summary_map[k],
            None => "",
        };
        println!("{} \t{}", b, summary);
    }
}
