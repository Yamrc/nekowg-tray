use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;

pub(crate) fn encode_wide<S: AsRef<OsStr>>(string: S) -> Vec<u16> {
    OsStrExt::encode_wide(string.as_ref())
        .chain(std::iter::once(0))
        .collect()
}
