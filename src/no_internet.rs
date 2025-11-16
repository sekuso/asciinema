use anyhow::anyhow;

pub fn disabled() -> anyhow::Error {
    anyhow!("Network access disabled: functionality that contacts the open internet has been removed in this version of asciinema")
}