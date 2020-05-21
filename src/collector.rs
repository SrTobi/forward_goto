use std::cmp::Ordering;
use syn::*;
use std::cmp::{min, min_by_key};
use std::collections::{HashMap, HashSet, VecDeque, binary_heap::BinaryHeap};

pub type Level = u32;


pub struct Collector {
    next_lifetime_id: u32,
    level: Level,
    continuation_level: Level,
    index: usize,
    gotos: HashMap<Lifetime, (Level, usize)>,
    labels: HashSet<Lifetime>,
    current_continuation_label: Option<Lifetime>,
    continuations: HashMap<Lifetime, (Vec<Stmt>, Option<Lifetime>)>
}

impl Collector {
    pub fn new() -> Self {
        Self {
            next_lifetime_id: 0,
            level: 0,
            continuation_level: 0,
            gotos: HashMap::new(),
            labels: HashSet::new(),
            index: usize::max_value(),
            current_continuation_label: None,
            continuations: HashMap::new(),
        }
    }

    pub fn new_lifetime(&mut self) -> Lifetime {
        let name = format!("_continuation{}", self.next_lifetime_id);
        self.next_lifetime_id += 1;
        Lifetime {
            apostrophe: proc_macro2::Span::call_site(),
            ident: Ident::new(&name, proc_macro2::Span::call_site()),
        }
    }

    pub fn add_goto(&mut self, lifetime: Lifetime) {
        assert!(self.index < usize::max_value());
        if !self.gotos.contains_key(&lifetime) {
            self.gotos.insert(lifetime, (self.level, self.index));
        }
    }

    pub fn add_label(&mut self, lifetime: Lifetime) {
        assert!(!self.labels.contains(&lifetime));
        self.labels.insert(lifetime.clone());
        self.current_continuation_label = Some(lifetime);
        self.continuation_level = self.level;
    }

    pub fn cut(&mut self) -> CollectorCut<'_> {
        let labels = std::mem::replace(&mut self.labels, HashSet::new());
        let current_continuation_label = std::mem::replace(&mut self.current_continuation_label, None);
        let continuations = std::mem::replace(&mut self.continuations, HashMap::new());
        CollectorCut {
            collector: self,
            labels,
            current_continuation_label,
            continuations,
        }
    }

    pub fn enter(&mut self) -> CollectorEnter<'_> {
        self.enter_statement(self.index)
    }

    pub fn enter_statement(&mut self, index: usize) -> CollectorEnter<'_> {
        let prev_index = self.index;
        self.level += 1;
        self.index = index;

        CollectorEnter {
            collector: self,
            prev_index,
        }
    }

    fn leave_statement(&mut self, prev_index: usize) {
        assert!(self.level > 0);
        self.level -= 1;
        self.continuation_level = min(self.continuation_level, self.level);

        for (_, p) in &mut self.gotos.iter_mut() {
            *p = min_by_key(*p, (self.level, self.index), |p| p.0);
        }

        self.index = prev_index;
    }

    #[must_use]
    pub fn should_push_continuation(&self) -> bool {
        !self.labels.is_empty() && self.continuation_level >= self.level
    }

    #[must_use]
    pub fn push_continuation(&mut self, continuation: Vec<Stmt>) -> Lifetime {
        let current_continuation_label = match self.current_continuation_label.take() {
            Some(label) => label,
            None => {
                let lifetime = self.new_lifetime();

                for (_, (_, to)) in &mut self.continuations.iter_mut() {
                    *to = Some(lifetime.clone());
                }
    
                lifetime
            },
        };
        self.continuations.insert(current_continuation_label.clone(), (continuation, None));
        current_continuation_label
    }

    #[must_use]
    pub fn retrieve_continuations(&mut self) -> Option<(Lifetime, Vec<(Option<usize>, Lifetime, Vec<Stmt>, Lifetime)>)> {
        #[derive(Eq)]
        struct Cont {
            index: usize,
            label: Lifetime,
        };
        impl PartialEq for Cont {
            fn eq(&self, rhs: &Self) -> bool {
                self.index == rhs.index
            }
        }
        impl Ord for Cont {
            fn cmp(&self, other: &Cont) -> Ordering {
                self.index.cmp(&other.index).reverse()
            }
        }
        impl PartialOrd for Cont {
            fn partial_cmp(&self, other: &Cont) -> Option<Ordering> {
                Some(self.cmp(other))
            }
        }
        

        let labels_to_generate: Vec<Lifetime> = self.labels
            .iter()
            .filter(|l| self.gotos.get(&l).iter().any(|(lvl, _)| *lvl == self.level))
            .cloned()
            .collect();

        if labels_to_generate.is_empty() {
            return None
        }
        
        let mut queue = BinaryHeap::new();
        for label in labels_to_generate {
            self.labels.remove(&label);
            queue.push(
                Cont {
                    index: self.gotos.get(&label).expect("expected label to be in goto").1,
                    label: label
                }
            )
        }

        let end_label = self.current_continuation_label.take().unwrap_or_else(|| self.new_lifetime());
        
        let to_generate = {
            assert!(self.labels.is_empty());

            let mut to_generate = Vec::new();
            let mut visited = HashSet::new();

            while let Some(Cont { label, index }) = queue.pop() {
                if !visited.contains(&label) {
                    visited.insert(label.clone());

                    match &self.continuations.get(&label).and_then(|c| c.1.as_ref()) {
                        Some(label) => queue.push(
                            Cont {
                                index,
                                label: (*label).clone(),
                            }
                        ),
                        None => (),
                    }

                    to_generate.push((index, label));
                }
            }

            to_generate
        };


        let result_conts: Vec<_> = to_generate.into_iter().map(|(index, start)| {
            let index = self.gotos.remove(&start).map(|g| g.1);
            let (stmts, maybe_end) = match self.continuations.remove(&start) {
                Some(t) => t,
                None => (Vec::new(), None),
            };
            println!("take continuation: {}", stmts.len());

            (index, start, stmts, maybe_end.unwrap_or(end_label.clone()))
        }).collect();

        Some((end_label, result_conts))
    }
}

impl Drop for Collector{
    fn drop(&mut self) {
        assert!(self.gotos.is_empty());
        assert!(self.continuations.is_empty());
        assert!(self.labels.is_empty());
        assert!(self.current_continuation_label.is_none());
    }
}

pub struct CollectorEnter<'t> {
    collector: &'t mut Collector,
    prev_index: usize,
}

impl<'t> Drop for CollectorEnter<'t> {
    fn drop(&mut self) {
        Collector::leave_statement(self.collector, self.prev_index);
    }
}

impl <'t> std::ops::Deref for CollectorEnter<'t> {
    type Target = Collector;
    fn deref(&self) -> &Collector {
        self.collector
    }
}

impl<'t> std::ops::DerefMut for CollectorEnter<'t> {

    fn deref_mut(&mut self) -> &mut Collector {
        self.collector
    }
}

pub struct CollectorCut<'t> {
    collector: &'t mut Collector,

    labels: HashSet<Lifetime>,
    current_continuation_label: Option<Lifetime>,
    continuations: HashMap<Lifetime, (Vec<Stmt>, Option<Lifetime>)>,
}

impl<'t> Drop for CollectorCut<'t> {
    fn drop(&mut self) {
        assert!(self.collector.labels.is_empty());
        assert!(self.collector.current_continuation_label.is_none());
        assert!(self.collector.continuations.is_empty());
        self.collector.labels = std::mem::replace(&mut self.labels, Default::default());
        self.collector.current_continuation_label = std::mem::replace(&mut self.current_continuation_label, Default::default());
        self.collector.continuations = std::mem::replace(&mut self.continuations, Default::default());
    }
}

impl <'t> std::ops::Deref for CollectorCut<'t> {
    type Target = Collector;
    fn deref(&self) -> &Collector {
        self.collector
    }
}

impl<'t> std::ops::DerefMut for CollectorCut<'t> {

    fn deref_mut(&mut self) -> &mut Collector {
        self.collector
    }
}
