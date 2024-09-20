
pub fn set_signal_handler() {
  ctrlc::set_handler(|| {
    disable_raw_mode!();

    std::process::exit(0);
  })
    .expect("Error setting Ctrl-C handler")
}
