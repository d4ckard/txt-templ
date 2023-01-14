use once_cell::sync::Lazy;

pub static LOGGING: Lazy<()> = Lazy::new(|| {
    env_logger::init();
});

