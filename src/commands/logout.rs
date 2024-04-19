use crate::conf::Conf;
use crate::fs::Fs;
use anyhow::Result;

pub fn handle<F: Fs>(conf: Conf, forget_dev: bool, fs: &F) -> Result<()> {
    let mut new_conf = Conf {
        session: None,
        ..conf
    };
    if forget_dev {
        new_conf.device_ids.clear()
    }
    new_conf.try_save(fs)
}
