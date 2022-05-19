use dec::{Decimal, D128};
use std::{collections::VecDeque};

#[derive(Clone, Debug)]
pub struct RegularStats {
    pub length: D128,
    history: VecDeque<(u64, D128)>,
    pub sum_dependent: D128,
    pub squared_sum_dependent: D128,
    pub sum_independent: D128,
    pub squared_sum_independent: D128,
    pub independent_mean: D128,
    pub sum_product_vars: D128, // this just holds independent times dependent
    pub sum_product_avg: D128,
    pub slope: D128, // slope of the linreg
    pub mean: D128,
    pub variance: D128,
    pub stan_dev: D128,
    pub last: D128,
    pub current: D128,
}

impl RegularStats {
    pub fn new() -> RegularStats {
        return RegularStats {
            length: D128::ZERO,
            history: VecDeque::new(),
            sum_dependent: D128::ZERO,
            squared_sum_dependent: D128::ZERO,
            mean: D128::ZERO,
            variance: D128::ZERO,
            stan_dev: D128::ZERO,
            sum_independent: D128::ZERO,
            squared_sum_independent: D128::ZERO,
            independent_mean: D128::ZERO,
            sum_product_vars: D128::ZERO,
            sum_product_avg: D128::ZERO,
            slope: D128::ZERO,
            last: D128::ZERO,
            current: D128::ZERO,
            // first_diffs: vec![D128::ZERO],
        };
    }
    pub fn init(key: u64, val: D128) -> RegularStats {
        let mut st = RegularStats::new();
        st.add(key, val);
        return st;
    }
    fn process(&mut self) {
        self.mean = self.sum_dependent / self.length;
        self.variance = (self.squared_sum_dependent
            - ((self.sum_dependent * self.sum_dependent) / self.length))
            / (self.length - D128::ONE);
        if self.variance.is_negative() {
            self.variance = D128::ZERO;
            // println!("Negative variance");
        }
        self.stan_dev = self.variance.fast_sqrt();
        self.independent_mean = self.sum_independent / self.length;
        self.slope = (self.length * self.sum_product_vars
            - self.sum_independent * self.sum_dependent)
            / (self.length * self.squared_sum_independent
                - (self.sum_independent * self.sum_independent)); // (n * sum(xy) - (sum(x) * sum(y))) / (n * sum(x^2) - sum(x)^2)
    }
    pub fn add(&mut self, key: u64, value: D128) {
        let independent = key;
        self.last = self.current;
        self.current = value;
        self.length += 1;
        self.history.push_back((key, value));
        self.sum_dependent += value;
        self.squared_sum_dependent += value * value;
        self.sum_independent += independent;
        self.squared_sum_independent += independent * independent;
        self.sum_product_vars += independent * value;
        self.sum_product_avg = self.sum_product_vars / self.sum_dependent;
        // self.first_diffs.push(value - *self.first_diffs.last().unwrap_or(&D128::ZERO));
        self.process();
    }
    pub fn prune(&mut self, lt: u64) {
        let drain_end = self
            .history
            .partition_point(|(timestamp, _)| timestamp < &lt);
        for (timestamp, value) in self.history.drain(..drain_end) {
            let independent = timestamp;
            self.length -= 1;
            self.sum_dependent -= value;
            self.squared_sum_dependent -= value * value;
            self.sum_independent -= independent;
            self.squared_sum_independent -= independent * independent;
            self.sum_product_vars -= independent * value;
        }
        self.process();
    }
}

pub struct NormalStats {
    pub length: D128,
    pub sum: D128,
    pub squared_sum: D128,
    pub slope: D128, // slope of the linreg
    pub mean: D128,
    pub variance: D128,
    pub stan_dev: D128,
    pub log_mean: D128,
    pub log_var: D128,
    pub log_stdv: D128,
    pub last: D128,
    pub highest: D128,
    pub lowest: D128,
    // pub hist: Vec<D128>,
    // pub first_diffs: Vec<D128>,
}

impl NormalStats {
    pub fn new() -> NormalStats {
        return NormalStats {
            length: D128::ZERO,
            sum: D128::ZERO,
            squared_sum: D128::ZERO,
            mean: D128::ZERO,
            variance: D128::ZERO,
            stan_dev: D128::ZERO,
            slope: D128::ZERO,
            last: D128::ZERO,
            highest: D128::ZERO,
            lowest: 1 / D128::ZERO,
            log_mean: D128::ZERO,
            log_var: D128::ZERO,
            log_stdv: D128::ZERO,
            // hist: vec![],
            // first_diffs: vec![D128::ZERO],
        };
    }
    pub fn init(_key: i64, val: D128) -> NormalStats {
        let mut st = NormalStats::new();
        st.add(val);
        return st;
    }
    pub fn process(&mut self) {
        self.mean = self.sum / self.length;
        self.variance = (self.squared_sum - ((self.sum * self.sum) / self.length))
            / (self.length - D128::ONE);
        if self.variance.is_negative() {
            self.variance = D128::ZERO;
            // eprintln!("Negative variance");
        }

        // self.stan_dev = NormalStats::sqrt(self.variance);

        // let timer = Instant::now();
        self.stan_dev = self.variance.sqrt();

        let meansqr = self.mean * self.mean;
        let _logmean = (meansqr / (meansqr + self.variance).fast_sqrt()).fast_ln();
        let logvar = (D128::ONE + (self.variance / meansqr)).fast_ln();
        let _logdev = logvar.fast_sqrt();
        // println!("mean: {}, var: {}, stdv: {}", )
    }
    pub fn add(&mut self, value: D128) {
        self.last = value;
        self.length += D128::ONE;
        self.sum += value;
        // println!("sum: {}, length: {}", self.sum, self.length);
        self.squared_sum += value * value;
        // self.first_diffs.push(value - *self.first_diffs.last().unwrap_or(&D128::ZERO));
        if self.highest < value {
            self.highest = value;
        }
        if self.lowest > value {
            self.lowest = value;
        }
        // self.hist.push(value);
        self.process();
    }
}
