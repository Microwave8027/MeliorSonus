use crate::utils::errors::RustError;

#[uniffi::export(callback_interface)]
pub trait ErrorCallback: Send + Sync + 'static {
    fn on_error(&self, msg: RustError);
    fn on_complete(&self);
}
