#[cfg(test)]
mod tests {
    use glob::glob;
    #[test]
    fn test_glob() {
        for entry in glob("tests/**/*.lua").expect("failed to read glob patterns") {
            match entry {
                Ok(path) => println!("{:?}", path.display()),
                Err(e) => println!("{:?}", e),
            }
        }
    }
}
