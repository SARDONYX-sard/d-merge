//! Finds the patch files `parse_nemesis_patch` does not come back from, with the hacks the merge uses.
//!
//! Run with `PATCH_EDITOR_MODS` = a folder of mods, e.g. MO2's `mods`.
use std::{
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    time::{Duration, Instant},
};

use fast_hash as _;
use imara_diff as _;
use json_patch as _;
use nemesis_diff as _;
use nemesis_xml::{hack::HackOptions, patch::parse_nemesis_patch};

/// How long one file may take before it counts as stuck. A debug build needs far less for any file.
const STUCK_AFTER: Duration = Duration::from_secs(20);

/// Every `#*.txt` under `dir`.
fn patch_files(dir: &Path, found: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for path in entries.flatten().map(|entry| entry.path()) {
        if path.is_dir() {
            patch_files(&path, found);
        } else if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('#') && name.ends_with(".txt"))
        {
            found.push(path);
        }
    }
}

#[test]
#[ignore = "needs real mods"]
fn no_patch_file_stalls_the_parser() {
    let mods = std::env::var("PATCH_EDITOR_MODS").unwrap();
    let mut files = Vec::new();
    for mod_dir in std::fs::read_dir(&mods).unwrap().flatten().map(|entry| entry.path()) {
        for engine in ["Nemesis_Engine", "Nemesis_EngineExt"] {
            patch_files(&mod_dir.join(engine).join("mod"), &mut files);
        }
    }
    println!("{} files", files.len());

    let workers = std::thread::available_parallelism().map_or(4, std::num::NonZero::get);
    let next = AtomicUsize::new(0);
    let done = AtomicUsize::new(0);
    let current: Vec<Mutex<Option<(Instant, usize)>>> = (0..workers).map(|_| Mutex::new(None)).collect();
    let slowest = Mutex::new((Duration::ZERO, 0_usize));
    let rejected = Mutex::new(Vec::new());

    std::thread::scope(|scope| {
        for slot in &current {
            let (files, next, done, slowest, rejected) = (&files, &next, &done, &slowest, &rejected);
            // Not joined when one is stuck: the watchdog below ends the process.
            scope.spawn(move || {
                while let Some(path) = files.get(next.fetch_add(1, Ordering::Relaxed)) {
                    let at = next.load(Ordering::Relaxed).saturating_sub(1);
                    let start = Instant::now();
                    *slot.lock().unwrap() = Some((start, at));
                    let bytes = std::fs::read(path).unwrap();
                    let text = String::from_utf8_lossy(&bytes);
                    let result = parse_nemesis_patch(&text, Some(HackOptions::enable_all()));
                    if result.is_err() {
                        rejected.lock().unwrap().push(at);
                    }
                    *slot.lock().unwrap() = None;
                    let took = start.elapsed();
                    {
                        let mut slowest = slowest.lock().unwrap();
                        if took > slowest.0 {
                            *slowest = (took, at);
                        }
                    }
                    done.fetch_add(1, Ordering::Relaxed);
                }
            });
        }
        // The watchdog.
        loop {
            std::thread::sleep(Duration::from_secs(2));
            let finished = done.load(Ordering::Relaxed);
            if finished >= files.len() {
                break;
            }
            let stuck: Vec<usize> = current
                .iter()
                .filter_map(|slot| slot.lock().ok().and_then(|slot| *slot))
                .filter(|(start, _)| start.elapsed() > STUCK_AFTER)
                .map(|(_, at)| at)
                .collect();
            println!("{finished} / {}", files.len());
            if !stuck.is_empty() {
                for at in stuck {
                    println!("STUCK: {}", files[at].display());
                }
                std::process::exit(3);
            }
        }
    });
    let (took, at) = *slowest.lock().unwrap();
    println!("all parsed; slowest {took:?}: {}", files[at].display());
    let rejected = rejected.lock().unwrap();
    println!("{} rejected", rejected.len());
    for &at in rejected.iter().take(40) {
        println!("REJECTED: {}", files[at].display());
    }
}
