use syn::*;
use std::cmp::{min, max};
use std::collections::{HashMap, HashSet};
use fix_fn::fix_fn;
use super::result::{ErrInfo, Result, err};
use syn::spanned::Spanned;

pub type Level = u32;

type ContinuationMap = HashMap<Lifetime, (Vec<Stmt>, Vec<Lifetime>)>;

pub struct Collector {
    next_label_id: u32,
    level: Level,
    continuation_level: Level,
    index: usize,
    gotos: HashMap<Lifetime, (Level, usize)>,
    labels: HashSet<Lifetime>,
    continuations: ContinuationMap,
    prev_conts: Vec<Lifetime>,
    errors: Vec<(ErrInfo, u32)>,
}

impl Collector {
    pub fn new() -> Self {
        Self {
            next_label_id: 0,
            level: 0,
            continuation_level: 0,
            gotos: HashMap::new(),
            labels: HashSet::new(),
            index: usize::max_value(),
            continuations: HashMap::new(),
            prev_conts: Vec::new(),
            errors: Vec::new(),
        }
    }

    fn new_lifetime(&mut self) -> Lifetime {
        let name = format!("_continuation{}", self.next_label_id);
        self.next_label_id += 1;
        Lifetime {
            apostrophe: proc_macro2::Span::call_site(),
            ident: Ident::new(&name, proc_macro2::Span::call_site()),
        }
    }

    pub fn add_goto(&mut self, label: Lifetime) {
        assert!(self.index < usize::max_value());
        if !self.gotos.contains_key(&label) {
            self.gotos.insert(label, (self.level, self.index));
        }
    }

    pub fn add_label(&mut self, label: Lifetime) -> Result<()> {
        if !self.gotos.contains_key(&label) {
            return err(label, "Found no goto to this label!")
        }
        if self.labels.contains(&label) {
            return err(label, "Label already used")
        }

        debug_assert!(self.prev_conts.is_empty());
        self.labels.insert(label.clone());
        self.prev_conts.push(label);
        self.continuation_level = self.level;

        Ok(())
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
            if self.level < p.0 {
                *p = (self.level, self.index);
            }
        }

        self.index = prev_index;
        self.prev_conts.extend(continuations);
    }

    #[must_use]
    pub fn should_push_continuation(&self) -> bool {
        !self.labels.is_empty() && self.continuation_level >= self.level
    }

    pub fn push_continuation(&mut self, continuation: Vec<Stmt>) -> Lifetime {
        assert!(!self.prev_conts.is_empty());
        let incoming_label = self.prev_conts[0].clone();

        if continuation.is_empty() && self.prev_conts.len() == 1 {
            return incoming_label;
        }
        
        let out_label = self.new_lifetime();

        let previous_continuations = std::mem::replace(
            &mut self.prev_conts,
            vec![out_label.clone()],
        );
        //eprint!("{} -> ", &out_label);
        //for p in previous_continuations.iter() {
        //    eprint!("{}", p);
        //}
        //eprintln!();

        self.continuations.insert(out_label, (continuation, previous_continuations));
        incoming_label
    }

    #[must_use]
    pub fn retrieve_continuations(&mut self) -> Option<(usize, Lifetime, Vec<(Vec<Lifetime>, Vec<Stmt>, Lifetime)>)> {
        let found_gotos_to_all_labels = self.labels
            .iter()
            .all(|l| self.gotos.get(&l).iter().any(|(lvl, _)| *lvl == self.level));

        if !found_gotos_to_all_labels || self.labels.is_empty() {
            return None
        }


        let mut gotos_to_generate: HashMap<Lifetime, usize> = HashMap::new();
        let mut largest_index = 0;
        let mut smallest_index = usize::MAX;

        for label in self.labels.drain() {
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
            |rec, cur: &Lifetime, result: &mut Vec<Lifetime>| -> () {
                match continuations.get(cur) {
                    Some((_, prevs)) => {
                        for p in prevs {
                             rec(p, result);
                        }

                        result.push(cur.clone());
                    },
                    None => ()
                }
            }
        );

        let sorted_conts_to_generate = {
            let mut conts_to_generate = Vec::new();
            rec(&end_label, &mut conts_to_generate);
            //conts_to_generate.sort_by_key(|e| usize::MAX - e.0);
            conts_to_generate
        };

        let mut result = Vec::new();

        for label in sorted_conts_to_generate {
            let (stmts, prevs) = self.continuations.remove(&label).unwrap();
            result.push((prevs, stmts, label));
        }

        Some((smallest_index, end_label, result))
    }

    pub fn check(mut self) -> Result<()> {
        for (goto, _) in self.gotos.drain() {
            if !self.labels.contains(&goto) {
                self.errors.push(((goto.span(), "Could not find target label!".into()), 1));
            }
        }

        for label in self.labels.drain() {
            self.errors.push(((label.span(), "Found no goto to this label!".into()), 0));
        }

        let mut errors = std::mem::replace(&mut self.errors, Vec::new());
        errors.sort_by_key(|(_, p)| *p);

        /*for ((_, e), _) in errors.iter() {
            eprintln!("Err: {}", e);
        }*/

        errors.first().map_or(Ok(()), |(info, _)| Err(info.clone()))
    }

    pub fn add_error(&mut self, span: impl Spanned, msg: impl Into<String>) {
        self.errors.push(((span.span(), msg.into()), 5));
    }
}

impl Drop for Collector {
    fn drop(&mut self) {
        assert!(self.labels.is_empty());
        assert!(self.gotos.is_empty());
        self.continuations.clear();
        self.prev_conts.clear();
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
        let collector = &mut self.collector;

        for label in collector.labels.drain() {
            collector.errors.push(((label.span(), "Found no goto to this label! Note that gotos cannot jump into expressions that need to provide a result value.".into()), 0));
        }

        collector.labels = std::mem::replace(&mut self.labels, Default::default());
        collector.prev_conts = std::mem::replace(&mut self.prev_conts, Default::default());
        collector.continuations = std::mem::replace(&mut self.continuations, Default::default());
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
