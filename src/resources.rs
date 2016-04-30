extern crate xml;

use std::env;
use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;
use std::io::{BufReader, BufWriter};
use std::process::Command;
use self::xml::reader::{EventReader, ParserConfig, XmlEvent};
use self::xml::reader::Error as XmlError;

const MANIFEST_SUFFIX: &'static str = ".gresource.xml";
const RESOURCE_SUFFIX: &'static str = ".gresource";
const RUST_SUFFIX: &'static str = "_resources.rs";

pub fn compile(name: &str) -> Result<(), Box<Error>> {
    let manifest_name = format!("{}{}", name, MANIFEST_SUFFIX);
    try!(check_inputs(&manifest_name).map_err(|e| {
        CustomError::new(e, format!("reading resource manifest `{}`", manifest_name))
    }));
    let out_dir = env::var("OUT_DIR").unwrap();
    let target_arg = format!("--target={}/{}{}", out_dir, name, RESOURCE_SUFFIX);
    let mut cmd = Command::new("glib-compile-resources");
    cmd.args(&[&target_arg, &manifest_name]);
    match cmd.output() {
        Ok(ref output) if output.status.success() => {},
        Ok(output) => {
            return Err(format!("Process didn't exit successfully: `{:?}`\n--- stderr\n{}", cmd,
                String::from_utf8_lossy(&output.stderr)).into())
        }
        Err(e) => return Err(CustomError::new(e, format!("running `{:?}`", cmd)).into()),
    }
    try!(codegen(&out_dir, name).map_err(|e| {
        CustomError::new(e, format!("writing rust module to `{}/{}{}`", out_dir, name, RUST_SUFFIX))
    }));
    Ok(())
}

#[derive(Debug)]
struct CustomError {
    cause: Box<Error>,
    note: String,
}

impl CustomError {
    fn new<T: Into<Box<Error>>>(cause: T, note: String) -> Self {
        CustomError { cause: cause.into(), note: note }
    }
}

impl fmt::Display for CustomError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!("{} {}", self.cause, self.note))
    }
}

impl Error for CustomError {
    fn description(&self) -> &str {
        self.cause.description()
    }

    fn cause(&self) -> Option<&Error> {
        Some(&*self.cause)
    }
}

fn check_inputs(manifest_name: &str) -> Result<(), Box<Error>> {
    let mut names = Vec::new();
    names.push(manifest_name.to_string());
    let file = BufReader::new(try!(File::open(manifest_name)));
    let mut parser = EventReader::new_with_config(file, ParserConfig {
        trim_whitespace: true,
        whitespace_to_characters: true,
        cdata_to_characters :true,
        ignore_comments: true,
        coalesce_characters: true,
    });
    try!(read_manifest(&mut parser, &mut names));
    for name in names {
        println!("cargo:rerun-if-changed={}", name);
    }
    Ok(())
}

fn codegen(out_dir: &str, name: &str) -> Result<(), Box<Error>> {
    let resource = try!(File::open(format!("{}/{}{}", out_dir, name, RESOURCE_SUFFIX)));
    let size = try!(resource.metadata()).len();
    let resource = BufReader::new(resource);
    let mut rust = BufWriter::new(
        try!(File::create(format!("{}/{}{}", out_dir, name, RUST_SUFFIX))));
    try!(write!(rust, r#"/// Registers the embedded resources.
///
/// You can include it in your code like this:
/// ```ignore
/// mod resources {{
///     include!(concat!(env!("OUT_DIR"), "/exampleapp_resources.rs"));
/// }}
/// ```
/// and then call `resources::register()`.
///
/// Requires explicit dependencies on `glib` and `gio` in `Cargo.toml`.
pub fn register() {{
    extern crate glib;
    extern crate gio;

    let bytes = glib::Bytes::from_static(&DATA.data);
    let resource = gio::Resource::new_from_data(&bytes).expect("corrupted embedded resources");
    gio::resources_register(&resource);

    struct Data {{
        data: [u8; {size}],
        _align: [f64; 0],
    }}

    static DATA: Data = Data {{
        data: [
           "#, size = size));

    for (n, byte) in resource.bytes().enumerate() {
        try!(write!(rust, " 0x{:02x},", try!(byte)));
        if (n + 1) % 8 == 0 {
            try!(write!(rust, "\n           "));
        }
    }

    try!(write!(rust, "{}", "
        ],
        _align: [],
    };
}
"));

    Ok(())
}

type Reader = EventReader<BufReader<File>>;

macro_rules! mk_error {
    ($pos:expr, $msg:expr) => (
        try!(Err((&*$pos, $msg)))
    )
}

fn read_manifest(parser: &mut Reader, names: &mut Vec<String>) -> Result<(), XmlError> {
    loop {
        match try!(parser.next()) {
            XmlEvent::StartDocument { .. } => continue,
            XmlEvent::EndDocument => return Ok(()),
            XmlEvent::StartElement { name, .. } => {
                match &*name.local_name {
                    "gresources" => try!(read_gresources(parser, names)),
                    elem => mk_error!(parser, format!("unexpected element <{}>", elem)),
                }
            }
            XmlEvent::Characters(_) => mk_error!(parser, "unexpected data"),
            _ => mk_error!(parser, "malformed XML")
        }
    }
}

fn read_gresources(parser: &mut Reader, names: &mut Vec<String>) -> Result<(), XmlError> {
    loop {
        match try!(parser.next()) {
            XmlEvent::StartElement { name, .. } => {
                match &*name.local_name {
                    "gresource" => try!(read_gresource(parser, names)),
                    elem => mk_error!(parser, format!("unexpected element <{}>", elem)),
                }
            }
            XmlEvent::EndElement { .. } => return Ok(()),
            XmlEvent::Characters(_) => mk_error!(parser, "unexpected data"),
            XmlEvent::EndDocument => mk_error!(parser, "unexpected EOF"),
            _ => mk_error!(parser, "malformed XML")
        }
    }
}

fn read_gresource(parser: &mut Reader, names: &mut Vec<String>) -> Result<(), XmlError> {
    loop {
        match try!(parser.next()) {
            XmlEvent::StartElement { name, .. } => {
                match &*name.local_name {
                    "file" => names.push(try!(read_data(parser))),
                    elem => mk_error!(parser, format!("unexpected element <{}>", elem)),
                }
            }
            XmlEvent::EndElement { .. } => return Ok(()),
            XmlEvent::Characters(_) => mk_error!(parser, "unexpected data"),
            XmlEvent::EndDocument => mk_error!(parser, "unexpected EOF"),
            _ => mk_error!(parser, "malformed XML")
        }
    }
}

fn read_data(parser: &mut Reader) -> Result<String, XmlError> {
    let mut ret = None;
    loop {
        match try!(parser.next()) {
            XmlEvent::Characters(s) => ret = Some(s),
            XmlEvent::EndElement { .. } => {
                if let Some(s) = ret {
                    return Ok(s)
                } else {
                    mk_error!(parser, "missing data")
                }
            }
            XmlEvent::StartElement { name, .. } => {
                mk_error!(parser, format!("unexpected element <{}>", name.local_name))
            }
            XmlEvent::EndDocument => mk_error!(parser, "unexpected EOF"),
            _ => mk_error!(parser, "malformed XML"),
        }
    }
}
