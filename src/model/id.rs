use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdGen {
    next: Id,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Id(u64);

impl IdGen {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn gen(&mut self) -> Id {
        let id = self.next;
        self.next.0 += 1;
        id
    }
}

// impl Id {
//     pub fn raw(&self) -> u64 {
//         self.0
//     }
// }

impl Default for IdGen {
    fn default() -> Self {
        Self { next: Id(0) }
    }
}
