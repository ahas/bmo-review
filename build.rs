// The migrations the store embeds at build time: a new one must rebuild the binary, or a cached build ships
// without it.
fn main() {
  println!("cargo:rerun-if-changed=migrations");
}
