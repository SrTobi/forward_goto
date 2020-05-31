use std::cmp::Ordering;
use syn::*;
use std::cmp::{min, max, min_by_key};
use std::collections::{HashMap, HashSet, VecDeque, binary_heap::BinaryHeap};
use fix_fn::fix_fn;

pub type Level = u32;

type ContinuationMap = HashMap<Lifetime, (Vec<Stmt>, Vec<Lifetime>)>;

pub struct Collector {
    next_lifetime_id: u32,
    level: Level,
    continuation_level: Level,
    index: usize,
    gotos: HashMap<Lifetime, (Level, usize)>,
    labels: HashSet<Lifetime>,
    continuations: ContinuationMap,
    prev_conts: Vec<Lifetime>,
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
            continuations: HashMap::new(),
            prev_conts: Vec::new(),
        }
    }

    fn new_lifetime(&mut self) -> Lifetime {
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
        debug_assert!(self.prev_conts.is_empty());
        self.labels.insert(lifetime.clone());
        self.prev_conts.push(lifetime);
        self.continuation_level = self.level;
    }

    pub fn cut(&mut self) -> CollectorCut<'_> {
        let labels = std::mem::replace(&mut self.labels, HashSet::new());
        let prev_conts = std::mem::replace(&mut self.prev_conts, Vec::new());
        let continuations = std::mem::replace(&mut self.continuations, HashMap::new());
        CollectorCut {
            collector: self,
            labels,
            prev_conts,
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

        let prev_conts = std::mem::replace(&mut self.prev_conts, Vec::new());
        CollectorEnter {
            collector: self,
            prev_index,
            prev_conts,
        }
    }

    fn leave_statement(&mut self, prev_index: usize, continuations: Vec<Lifetime>) {
        assert!(self.level > 0);
        self.level -= 1;
        self.continuation_level = min(self.continuation_level, self.level);

        for (_, p) in &mut self.gotos.iter_mut() {
            *p = min_by_key(*p, (self.level, self.index), |p| p.0);
        }

        self.index = prev_index;
        self.prev_conts.extend(continuations);
    }

    #[must_use]
    pub fn should_push_continuation(&self) -> bool {
        !self.labels.is_empty() && self.continuation_level >= self.level
    }

    #[must_use]
    pub fn push_continuation(&mut self, continuation: Vec<Stmt>) -> Lifetime {
        assert!(!self.prev_conts.is_empty());
        
        let lifetime = self.new_lifetime();

        let previous_continuations = std::mem::replace(
            &mut self.prev_conts,
            vec![lifetime.clone()],
        );

        self.continuations.insert(lifetime.clone(), (continuation, previous_continuations));
        lifetime
    }

    #[must_use]
    pub fn retrieve_continuations(&mut self) -> Option<(usize, Lifetime, Vec<(usize, Vec<Lifetime>, Vec<Stmt>, Lifetime)>)> {
        let labels_to_generate: Vec<Lifetime> = self.labels
            .iter()
            .filter(|l| self.gotos.get(&l).iter().any(|(lvl, _)| *lvl == self.level))
            .cloned()
            .collect();

        if labels_to_generate.is_empty() {
            return None
        }


        let mut gotos_to_generate: HashMap<Lifetime, usize> = HashMap::new();
        let mut largest_index = 0;
        let mut smallest_index = usize::MAX;

        for label in labels_to_generate {
            self.labels.remove(&label);
            let (_, index) = self.gotos.remove(&label).expect("'label' should be in self.goto");
            gotos_to_generate.insert(label, index);
            largest_index = max(largest_index, index);
            smallest_index = min(smallest_index, index);
        }


        for (_, (lvl, index)) in &mut self.gotos.iter_mut() {
            if *lvl == self.level {
                *index = min(*index, smallest_index);
            }
        }

        debug_assert!(!self.prev_conts.is_empty());
        if self.prev_conts.len() > 1 {
            self.push_continuation(Vec::new());
        }
        debug_assert!(self.prev_conts.len() == 1);
        let end_label = self.prev_conts.drain(..).next().unwrap();
        
        let continuations = &self.continuations;
        let rec = fix_fn!(
            |rec, cur: &Lifetime, index: usize, result: &mut Vec<(usize, Lifetime)>| -> bool {
                let index = *gotos_to_generate.get(cur).unwrap_or(&index);
                match continuations.get(cur) {
                    Some((_, prevs)) => {
                        let mut found_unrelated_label = false;
                        for p in prevs {
                             found_unrelated_label |= rec(p, index, result);
                        }

                        if !found_unrelated_label {
                            result.push((index, cur.clone()));
                        }

                        found_unrelated_label
                    },
                    None => {
                        false
                    }
                }
            }
        );

        let sorted_conts_to_generate = {
            let mut conts_to_generate = Vec::new();
            rec(&end_label, largest_index, &mut conts_to_generate);
            conts_to_generate.sort_by_key(|e| usize::MAX - e.0);
            conts_to_generate
        };

        let mut result = Vec::new();

        for (index, label) in sorted_conts_to_generate {
            let (stmts, prevs) = self.continuations.remove(&label).unwrap();
            result.push((index, prevs, stmts, label));
        }

        Some((largest_index, end_label, result))

        /*let to_generate = {
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

        Some((end_label, result_conts))*/
        //todo!()
    }
}

impl Drop for Collector{
    fn drop(&mut self) {
        assert!(self.gotos.is_empty());
        assert!(self.continuations.is_empty());
        assert!(self.labels.is_empty());
        assert!(self.prev_conts.is_empty());
    }
}

pub struct CollectorEnter<'t> {
    collector: &'t mut Collector,
    prev_index: usize,
    prev_conts: Vec<Lifetime>,
}

impl<'t> Drop for CollectorEnter<'t> {
    fn drop(&mut self) {
        let continuations = std::mem::replace(&mut self.prev_conts, Default::default());
        Collector::leave_statement(self.collector, self.prev_index, continuations);
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
    prev_conts: Vec<Lifetime>,
    continuations: HashMap<Lifetime, (Vec<Stmt>, Vec<Lifetime>)>,
}

impl<'t> Drop for CollectorCut<'t> {
    fn drop(&mut self) {
        assert!(self.collector.labels.is_empty());
        assert!(self.collector.prev_conts.is_empty());
        assert!(self.collector.continuations.is_empty());
        self.collector.labels = std::mem::replace(&mut self.labels, Default::default());
        self.collector.prev_conts = std::mem::replace(&mut self.prev_conts, Default::default());
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
