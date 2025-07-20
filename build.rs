fn main() {
    println!("cargo:rustc-link-lib=ncurses");
    println!("cargo:rustc-link-lib=tinfo");
    println!("cargo:rustc-link-lib=gcc_s");
    println!("cargo:rustc-link-lib=m");
    println!("cargo:rustc-link-lib=c");
}