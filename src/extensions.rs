pub trait LineSplitter {
    fn split_lines(&self) -> Vec<String>;
}

impl LineSplitter for &str {
    fn split_lines(&self) -> Vec<String> {
        self.split('\n').map(ToString::to_string).collect()
    }
}
