#[cfg(feature = "logger")]
fn init() {
    let log_file = File::options()
        .create(true)
        .append(true)
        .open("app.log")
        .await?;

    Builder::new()
        .with_default_writer(new_writer(log_file))
        .init();
}
