use std::path::Path;
use std::process::Command;
use std::{env, fs};

const INCLUDES: &[&str] = &["cryptogams/arm"];

fn main() {
    let target_arch = env("CARGO_CFG_TARGET_ARCH");
    let target_features = env("CARGO_CFG_TARGET_FEATURE");
    let target_features = target_features.split(',').collect::<Vec<_>>();
    let feature = |s: &str| target_features.iter().any(|&f| f == s);

    let script = match target_arch.as_str() {
        "x86" => "cryptogams/x86/keccak1600-mmx.pl",
        "x86_64" => {
            if feature("avx512vl") {
                "cryptogams/x86_64/keccak1600-avx512vl.pl"
            } else if feature("avx512f") {
                "cryptogams/x86_64/keccak1600-avx512.pl"
            } else if feature("avx2") {
                "cryptogams/x86_64/keccak1600-avx2.pl"
            } else {
                "cryptogams/x86_64/keccak1600-x86_64.pl"
            }
        }
        "aarch64" => {
            // TODO: I don't think this is right
            if feature("sha3") || feature("crypto") {
                "cryptogams/arm/keccak1600-armv8.pl"
            } else {
                "cryptogams/arm/keccak1600-armv4.pl"
            }
        }
        // TODO: ia64, mips, ppc, riscv, s390x in cryptogams/ all have keccak1600
        s => panic!("Unsupported target arch: {s}"),
    };
    let src = Path::new(script).file_stem().unwrap().to_str().unwrap();
    let sha3 = Path::new(&env("OUT_DIR")).join(format!("{src}.s"));
    println!("cargo:rustc-env=SHA3_ASM_SRC={src}");

    // perl scripts args
    let os = env("CARGO_CFG_TARGET_OS");
    let environ = env("CARGO_CFG_TARGET_ENV");
    let family = env("CARGO_CFG_TARGET_FAMILY");
    let mut flavor = match target_arch.as_str() {
        "aarch64" => match os.as_str() {
            "ios" => Some("ios64"),
            "windows" => Some("win64"),
            "linux" => Some("linux64"),
            _ => None,
        },
        "x86_64" => match os.as_str() {
            "macos" => Some("macosx"),
            "windows" if environ == "gnu" => Some("mingw64"),
            _ if family == "unix" => Some("elf"),
            _ => None,
        },
        "x86" => match os.as_str() {
            "windows" => Some("win32n"),
            _ => Some("elf"),
        },
        _ => None,
    }
    .map(String::from);
    if let Some(s) = &mut flavor {
        if target_arch == "aarch64" && feature("sha3") {
            s.push_str("+sha3");
        }
    }

    perl(script, flavor.as_deref(), sha3.to_str().unwrap());

    cc::Build::new().includes(INCLUDES).file(sha3).compile("keccak");
}

fn perl(path: &str, flavor: Option<&str>, to: &str) {
    let mut cmd = Command::new("perl");
    cmd.arg(path);
    if let Some(flavor) = flavor {
        cmd.arg(flavor);
    }
    cmd.arg(to);
    let out = cmd.output().unwrap();
    if !out.status.success() {
        panic!("perl for {path} failed:\n{}", String::from_utf8_lossy(&out.stderr));
    }
    let stdout = String::from_utf8(out.stdout).unwrap();
    if !stdout.trim().is_empty() {
        fs::write(to, stdout).unwrap();
    }
}

fn env(s: &str) -> String {
    println!("cargo:rerun-if-env-changed={s}");
    env::var(s).unwrap()
}
