pub mod alloy;
#[cfg(feature = "datadog")]
pub mod datadog;
pub mod grafana;
pub mod query;
pub mod scrape;
pub mod yaml;
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
