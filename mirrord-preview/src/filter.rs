use std::{collections::HashSet, str::FromStr};

#[derive(Default, Clone, Debug)]
pub struct FilterPorts {
    ranges: Vec<(u32, u32)>,
    specific: HashSet<u32>,
}

impl FilterPorts {
    pub fn is_match(&self, port: u32) -> bool {
        for (start, end) in &self.ranges {
            if port > *start && port < *end {
                return true;
            }
        }

        self.specific.contains(&port)
    }
}

impl FromStr for FilterPorts {
    type Err = <u32 as FromStr>::Err;

    fn from_str(source: &str) -> Result<Self, <Self as FromStr>::Err> {
        let mut filter = Self::default();

        for part in source.split(',') {
            if part.contains("..") {
                let part: Vec<&str> = part.splitn(2, "..").collect();

                filter.ranges.push((part[0].parse()?, part[1].parse()?));
            } else {
                filter.specific.insert(part.parse()?);
            }
        }

        Ok(filter)
    }
}
