mod base_station;
mod city;
mod district;
mod idc;
pub mod meta;

use anyhow::*;
pub use base_station::BaseStationInfo;
pub use city::CityInfo;
pub use district::DistrictInfo;
pub use idc::IdcInfo;
pub use meta::Meta;
use std::collections::BTreeMap;
use std::convert::TryInto;
use std::net::IpAddr;
use std::str::FromStr;

pub struct Reader<'d> {
    data: &'d [u8],
    meta: Meta,
    v4offset: usize,
}

#[cfg(feature = "internal-db")]
impl Reader<'static> {
    pub fn load() -> Result<Self> {
        let data = Box::new(include_bytes!("../../qqwry.ipdb"));
        Self::from_bytes(Box::leak(data).as_slice())
    }
}

impl<'d> Reader<'d> {
    pub fn from_bytes(data: &'d [u8]) -> Result<Reader<'d>> {
        let meta_length = u32::from_be_bytes(data[..4].try_into()?) as usize;
        let meta: Meta = serde_json::from_reader(&data[4..meta_length + 4])?;

        ensure!(
            4 + meta_length + meta.total_size == data.len(),
            "database file size error"
        );

        let data = &data[4 + meta_length..];
        let mut node = 0usize;
        for i in 0..96 {
            if node >= meta.node_count {
                break;
            }
            if i >= 80 {
                let off = node * 8 + 4;
                node = u32::from_be_bytes((&data[off..off + 4]).try_into()?) as usize;
            } else {
                let off = node * 8;
                node = u32::from_be_bytes((&data[off..off + 4]).try_into()?) as usize;
            }
        }

        Ok(Reader {
            data,
            meta,
            v4offset: node,
        })
    }

    fn resolve(&self, node: usize) -> Result<&str> {
        let resolved = node - self.meta.node_count + self.meta.node_count * 8;
        ensure!(
            resolved < self.data.len(),
            "database resolve error,resolved:{}>file length:{}",
            resolved,
            self.data.len()
        );
        let size = u32::from_be_bytes([0u8, 0u8, self.data[resolved], self.data[resolved + 1]])
            as usize
            + resolved
            + 2;
        ensure!(
            self.data.len() > size,
            "database resolve error,size:{}>file length:{}",
            size,
            self.data.len()
        );
        unsafe {
            Ok(std::str::from_utf8_unchecked(
                &self.data[resolved + 2..size],
            ))
        }
    }

    #[inline]
    fn read_node(&self, node: usize, index: usize) -> Result<usize> {
        let off = node * 8 + index * 4;
        Ok(u32::from_be_bytes((&self.data[off..off + 4]).try_into()?) as usize)
    }

    #[inline]
    fn find_node(&self, binary: &[u8]) -> Result<usize> {
        let mut node = 0;
        let bit = binary.len() * 8;
        if bit == 32 {
            node = self.v4offset;
        }
        for i in 0..bit {
            if node > self.meta.node_count {
                return Ok(node);
            }
            let byte = binary[i / 8];
            let bit_index = 7 - (i % 8);
            let bit = ((byte >> bit_index) & 1) as usize;
            node = self.read_node(node, bit)?;
        }

        if node > self.meta.node_count {
            Ok(node)
        } else {
            bail!("not found ip")
        }
    }

    #[inline(always)]
    pub fn is_ipv4(&self) -> bool {
        self.meta.ip_version & 0x01 == 0x01
    }

    #[inline(always)]
    pub fn is_ipv6(&self) -> bool {
        self.meta.ip_version & 0x02 == 0x02
    }

    #[inline]
    pub fn find(&self, addr: &str, language: &str) -> Result<Vec<&str>> {
        let addr = IpAddr::from_str(addr)?;
        ensure!(!self.meta.fields.is_empty(), "fields is empty");
        let off = *self
            .meta
            .languages
            .get(language)
            .ok_or_else(|| anyhow!("not found language:{}", language))?;
        let mut _ipv4_buff;
        let mut _ipv6_buff;
        let ipv = match &addr {
            IpAddr::V4(v) => {
                ensure!(self.is_ipv4(), "error:ipdb is ipv6");
                _ipv4_buff = v.octets();
                &_ipv4_buff[..]
            }
            IpAddr::V6(v) => {
                ensure!(self.is_ipv6(), "error:ipdb is ipv4");
                _ipv6_buff = v.octets();
                &_ipv6_buff[..]
            }
        };
        let node = self.find_node(ipv)?;
        let context = self.resolve(node)?;
        let sp: Vec<&str> = context.split('\t').skip(off).collect();
        Ok(sp)
    }

    #[inline]
    pub fn find_city_info<'s>(&'d self, addr: &'s str, language: &'s str) -> Result<CityInfo<'d>> {
        Ok(self.find(addr, language)?.into())
    }

    #[inline]
    pub fn find_district_info<'s>(
        &'d self,
        addr: &'s str,
        language: &'s str,
    ) -> Result<DistrictInfo<'d>> {
        Ok(self.find(addr, language)?.into())
    }

    #[inline]
    pub fn find_idc_info<'s>(&'d self, addr: &'s str, language: &'s str) -> Result<IdcInfo<'d>> {
        Ok(self.find(addr, language)?.into())
    }

    #[inline]
    pub fn find_base_station_info<'s>(
        &'d self,
        addr: &'s str,
        language: &'s str,
    ) -> Result<BaseStationInfo<'d>> {
        Ok(self.find(addr, language)?.into())
    }

    #[inline]
    pub fn find_map(&self, addr: &str, language: &str) -> Result<BTreeMap<&str, &str>> {
        let v = self.find(addr, language)?;
        let k = &self.meta.fields;
        let map = k
            .iter()
            .map(|k| k.as_str())
            .zip(v)
            .collect();
        Ok(map)
    }
}
