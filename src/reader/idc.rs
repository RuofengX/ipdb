use std::fmt::Display;
use serde::Serialize;
#[derive(Debug, Serialize)]
pub struct IdcInfo<'a> {
    pub country_name: &'a str,
    pub region_name: &'a str,
    pub city_name: &'a str,
    pub owner_domain: &'a str,
    pub isp_domain: &'a str,
    pub idc: &'a str,
}

impl<'a> From<Vec<&'a str>> for IdcInfo<'a> {
    fn from(buff: Vec<&'a str>) -> Self {
        IdcInfo {
            country_name: if !buff.is_empty() { buff[0] } else { "" },
            region_name: if buff.len() > 1 { buff[1] } else { "" },
            city_name: if buff.len() > 2 { buff[2] } else { "" },
            owner_domain: if buff.len() > 3 { buff[3] } else { "" },
            isp_domain: if buff.len() > 4 { buff[4] } else { "" },
            idc: if buff.len() > 5 { buff[5] } else { "" },
        }
    }
}

impl<'a> Display for IdcInfo<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        serde_json::to_string(self).unwrap().fmt(f)
    }
}
