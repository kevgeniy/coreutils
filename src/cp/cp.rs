#![crate_name = "uu_cp"]

// This file is part of the uutils coreutils package.
//
// (c) Jordy Dickinson <jordy.dickinson@gmail.com>
//
// For the full copyright and license information, please view the LICENSE file
// that was distributed with this source code.
//

#[macro_use]
extern crate uucore;
extern crate getopts;

use std::{env};
use std::io::{Error, ErrorKind};
use std::fs::{File, copy, hard_link, soft_link, canonicalize};
use std::path::{Path, PathBuf};
use std::vec::Vec;

static USAGE: &'static str = "[OPTION]... [-T] SOURCE DEST\n   [OPTION]... SOURCE... \
                              DEST\n   [OPTION]... -t DIRECTORY SOURCE...";
static SUMMARY: &'static str = "Copy SOURCE to DEST, or multiple SOURCE(s) to DIRECTORY\n \
                                Mandatory arguments to long options are mandatory for short \
                                options too.";
static LONG_HELP: &'static str = "";

pub fn uumain(args: Vec<String>) -> i32 {
    let matches = new_coreopts!(USAGE, SUMMARY, LONG_HELP)
        .optflag("a", "archive", "same as -dR --preserve=all")
        .optflag("",
                 "attributes-only",
                 "don't copy the file data, just the attributes")
        .optflagopt("",
                    "backup",
                    "make a backup of each existing destination file",
                    "")
        .optflag("b", "", "like --backup but does not accept an argument")
        .optflag("",
                 "copy-contents",
                 "copy contents of special files when recursive")
        .optflag("d", "", "same as --no-dereference --preserve=links")
        .optflag("f",
                 "force",
                 "if an existing destination file cannot be opened, remove it and try again \
                  (this option is ignored when the -n option is also used)")
        .optflag("i",
                 "interactive",
                 "prompt before overwrite (overrides a previous -n option)")
        .optflag("H", "", "follow command-line symbolic links in SOURCE")
        .optflag("l", "link", "hard link files instead of copying")
        .optflag("L", "dereference", "always follow symbolic links in SOURCE")
        .optflag("n",
                 "no-clobber",
                 "do not overwrite an  existing  file  (overrides  a  previous  -i option)")
        .optflag("P",
                 "no-dereference",
                 "never follow symbolic links in SOURCE")
        .optflag("p", "", "same as --preserve=mode,ownership,timestamps")
        .optflagopt("",
                    "preserve",
                    "preserve the specified attributes (default: mode,ownership,timestamps), if  \
                     possible  additional  attributes:  context,  links, xattr, all",
                    "")
        .optflag("",
                 "no-preserve",
                 "don't preserve the specified attributes")
        .optflag("", "parents", "use full source file name under DIRECTORY")
        .optflag("r", "recursive", "copy directories recursively")
        .optflag("R", "recursive", "copy directories recursively")
        .optflag("", "reflink=[WHEN]", "control clone/CoW copies. See below")
        .optflag("",
                 "remove-destination",
                 "remove  each existing destination file before attempting to open
              \
                  it (contrast with --force)")
        .optflag("",
                 "sparse=[WHEN]",
                 "control creation of sparse files. See below")
        .optflag("",
                 "strip-trailing-slashes",
                 "remove any trailing slashes from each SOURCE argument")
        .optflag("s",
                 "symbolic-link",
                 "make symbolic links instead of copying")
        .optopt("S", "suffix", "override the usual backup suffix", "")
        .optopt("t",
                "target-directory",
                "copy all SOURCE arguments into DIRECTORY",
                "")
        .optflag("T", "no-target-directory", "treat DEST as a normal file")
        .optflag("u",
                 "update",
                 "copy only when the SOURCE file is  newer  than  the  destination file or when \
                  the destination file is missing")
        .optflag("v", "verbose", "explain what is being done")
        .optflag("x", "one-file-system", "stay on this file system")
        .optflag("Z",
                 "",
                 "set SELinux security context of destination file to default type")
        .optflag("",
                 "context[=CTX]",
                 "like  -Z,  or  if CTX is specified then set the SELinux or SMACK security \
                  context to CTX")
        .parse(args);

    let (options, destination, mut sources) = setup_options(matches);

    // Suppose that options.recursive == false
    let mut files = Vec::with_capacity(sources.len());
    if options.deref != DerefMode::Never {
        for source in &mut sources {
            files.push(canonicalize(source).expect("TODO: ERROR MESSAGE"));
        }
    } else {
        for source in &mut sources {
            files.push(Path::new(source).to_path_buf());
        }
    }

    let dest_file = Path::new(&destination).to_path_buf();
    //let dest_file = canonicalize(destination.clone()).expect("ERROR MESSAGE");

    // 1. file to file
    // TODO: when destination is file and it doesn't exist and its parent doesn't exist -- error 
    let flag = !dest_file.exists() && dest_file.parent().unwrap().exists() || dest_file.is_dir() && options.dir_as_file;
    if flag {
        copy_file_to_file(options, dest_file, destination, files[0].clone(), sources);
    } else {
        0;
    }
    0
    // copy(options, destination, sources)

}


fn copy_file_to_file(options: Options,
                     dest_file: PathBuf,
                     destination: String,
                     file: PathBuf,
                     sources: Vec<String>)
                     -> Result<(), std::io::Error> {
    if sources.len() > 1 {
        if options.dir_as_file {
            if sources.len() > 2 {
                println!("cp: extra operand '{}'", sources[2]);
                return Err(Error::new(ErrorKind::Other, "-1"));
            }
            println!("cp: extra operand '{}'", destination);
            return Err(Error::new(ErrorKind::Other, "-1"));
        }
        println!("cp: target '{}' is not a directory", destination);
        return Err(Error::new(ErrorKind::Other, "-1"));
    }

    // declare and open the destination file
    if !dest_file.exists() {
        // нужно ли создавать целевой файл
        match options.copy {
            CopyMode::Attributes => { File::create(dest_file); },
            CopyMode::Data => { copy(file, dest_file); },
            CopyMode::Symlink => { soft_link(file, dest_file); },
            CopyMode::Hardlink => { hard_link(file, dest_file); },
        };
    } else {
        match options.overwrite {
            OverwriteMode::Rewrite => return Ok(()),
            OverwriteMode::Prompt => return Ok(()),
            OverwriteMode::Remove => return Ok(()),
            OverwriteMode::Force => return Ok(()),
            OverwriteMode::Update => return Ok(()),
            OverwriteMode::None => return Ok(()),
        };
    }

    // file => opened file
    // copy opened file to destination file 
    Ok(())
}

// --link и связь с deref линков
// copy-contents
// parents
// reflink
// sparse
// strip-trailing-slashes
// verbose
// one-file-system
// Z
// context

struct Options {
    recursive: bool,
    dir_as_file: bool,
    overwrite: OverwriteMode,
    copy: CopyMode,
    deref: DerefMode,
    backup: BackupMode,
    backup_suffix: String,
    preserve_attr: Vec<String>,
}

impl Options {
    fn new() -> Self {
        Options {
            recursive: false,
            dir_as_file: false,
            overwrite: OverwriteMode::Rewrite,
            copy: CopyMode::Data,
            deref: DerefMode::CommandLine,
            backup: BackupMode::None,
            backup_suffix: if let Ok(val) = env::var("SIMPLE_BACKUP_SUFFIX") {
                val
            } else {
                "~".to_owned()
            },
            preserve_attr: Vec::new(),
        }
    }
}


// как воспринимать существующий целевой файл
enum OverwriteMode {
    Rewrite,
    Prompt,
    Remove,
    Force,
    Update,
    None,
}

// что копировать: атрибуты, данные, создать ссылку
enum CopyMode {
    Attributes,
    Data,
    Symlink,
    Hardlink,
}

// как разрешать symlink в source
#[derive(PartialEq)]
enum DerefMode {
    Always,
    CommandLine,
    Never,
}

// backup режим
enum BackupMode {
    None,
    Numbered,
    Existing,
    Simple,
}

fn setup_options(matches: getopts::Matches) -> (Options, String, Vec<String>) {
    let mut options = Options::new();

    // select copy mode
    // TODO: error if there're more than one option at the same time
    if matches.opt_present("attributes-only") {
        options.copy = CopyMode::Attributes;
    };
    if matches.opt_present("l") {
        options.copy = CopyMode::Hardlink;
    };
    if matches.opt_present("s") {
        options.copy = CopyMode::Symlink;
    };

    // select dereference mode
    if matches.opts_present(&["r".to_owned(), "R".to_owned(), "a".to_owned(), "d".to_owned()]) {
        options.deref = DerefMode::Never;
    }
    // TODO: Обработать перекрытие поздней опцией более ранней
    if matches.opt_present("L") {
        options.deref = DerefMode::Always;
    } else if matches.opt_present("H") {
        options.deref = DerefMode::CommandLine;
    } else {
        options.deref = DerefMode::Never;
    }

    if matches.opts_present(&["r".to_owned(), "R".to_owned(), "a".to_owned()]) {
        options.recursive = true;
    }

    // backup is used when the destination file will be overwritten or removed
    // special case when --force and -b and SOURCE == TARGET and is regular file
    if matches.opt_present("b") {
        options.backup = BackupMode::Existing;
    }
    if matches.opt_present("backup") {
        let backup_str = match matches.opt_str("backup") {
            Some(val) => val,
            _ => {
                match env::var("VERSION_CONTROL") {
                    Ok(val) => val,
                    _ => "existing".to_owned(),
                }
            }
        };
        if backup_str == "none" || backup_str == "off" {
            options.backup = BackupMode::None;
        } else if backup_str == "numbered" || backup_str == "t" {
            options.backup = BackupMode::Numbered;
        } else if backup_str == "existing" || backup_str == "nil" {
            options.backup = BackupMode::Existing;
        } else {
            options.backup = BackupMode::Simple;
        }
    }
    if matches.opt_present("S") {
        options.backup_suffix = matches.opt_str("S").expect("option requires an argument -- 'S'");
    }

    // save preserve options
    if matches.opt_present("p") {
        options.preserve_attr = vec!["mode".to_owned()];
    } else if matches.opt_present("d") {
        options.preserve_attr = vec!["links".to_owned()];
    } else if matches.opt_present("a") {
        options.preserve_attr = vec!["all".to_owned()];
    } else if matches.opt_present("preserve") {
        options.preserve_attr.append(&mut matches.opt_strs("preserve"));
    }
    if matches.opt_present("no-preserve") {
        let values = matches.opt_strs("no-preserve");
        options.preserve_attr.retain(|elem| !values.contains(elem));
    }

    // TODO: Обработать перекрытие поздней опцией более ранней
    if matches.opt_present("f") {
        options.overwrite = OverwriteMode::Force;
    } else if matches.opt_present("i") {
        options.overwrite = OverwriteMode::Prompt;
    } else if matches.opt_present("n") {
        options.overwrite = OverwriteMode::None;
    } else if matches.opt_present("remove-destination") {
        options.overwrite = OverwriteMode::Remove;
    } else if matches.opt_present("u") {
        options.overwrite = OverwriteMode::Update;
    }

    if matches.opt_present("T") {
        options.dir_as_file = true;
    }

    let destination;
    let mut sources;// = Vec::new();
    if matches.opt_present("t") {
        destination = matches.opt_str("t").expect("option requires an argument -- 't'");
        sources = matches.free.clone();
    } else {
        let len = matches.free.len();
        destination = matches.free[len - 1].clone();
        sources = vec![String::new(); len - 1];
        sources.clone_from_slice(&matches.free[0..len - 1]);
    }
    (options, destination, sources)
}

// fn full_copy(options: Options, destination: String, sources: Vec<String>) -> i32 {
//     // process destination files (delete, prompt, ...)

//     // copy what need to copy

//     0
// }
