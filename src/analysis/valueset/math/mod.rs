// Copyright (c) 2015, The Radare Project. All rights reserved.
// See the COPYING file at the top-level directory of this distribution.
// Licensed under the BSD 3-Clause License:
// <http://opensource.org/licenses/BSD-3-Clause>
// This file may not be copied, modified, or distributed
// except according to those terms.

#[cfg(test)]
mod test;

// warning: rust uses '!' as bitwise not operator
// blcic(!x) = tzmsk(x)+1 = product of the '2's of x's prime decomposition

// 101111-> 10000
pub fn blcic(x: u64) -> u64 {
    x.wrapping_add(1) & !x
} 

// 010000->  1111
pub fn tzmsk(x: u64) -> u64 {
    x.wrapping_sub(1) & !x
} 

pub fn bitsmear(mut smear: u64) -> u64 {
    smear |= smear >> 32;
    smear |= smear >> 16;
    smear |= smear >> 8;
    smear |= smear >> 4;
    smear |= smear >> 2;
    smear |= smear >> 1;
    smear
}

pub fn gcd_lcm(mut m: u64, mut n: u64) -> (u64, u64) {
    let p = m * n;
    while m != 0 {
        let o = m;
        m = n % m;
        n = o;
    }
    (n,
     if n == 0 {
        0
    } else {
        p / n
    })
}

pub fn multiplicative_inverse(mut a: u64, n: u64) -> Option<u64> {

    if n == 0 {
        return Option::None;
    }
    a %= n;

    if a == 0 {
        return Option::None;
    }

    let mut t: u64 = 0;
    let mut r: u64 = n;
    let mut nt: u64 = 1;
    let mut nr: u64 = a;

    while nr != 0 {
        // TODO: make sure q*nt never overflows
        let (ot, or) = (nt, nr);
        let q = r / nr;

        // TODO: make sure this doesn't overflow
        nt = (t + q * (n - nt)) % n;
        nr = r - q * nr;
        t = ot;
        r = or;
    }
    if r > 1 {
        return Option::None;
    }

    Option::Some(t)
}

/// Traits for math operation of set.
pub trait set_theory {
    /// Return if Set A is a subset of Set B.
    fn has_subset(&self, _: &Self) -> bool;

    /// Return the intersection(meet) of Set A and Set B.
    fn meet(&self, _: &Self) -> Self;
    
    /// Return the union(join) of Set A and Set B.
    fn join(&self, _: &Self) -> Self;
    
    /// Widen Set A with respect to Set B.
    fn widen(&mut self, _: &Self);
    
    /// Adjust all value in Set A by a constant B.
    fn adjust(&mut self, _: i64);
    
    /// Set lower bound of Set A to negative infinitude.
    fn remove_lower_boundes(&mut self);
    
    /// Set upper bound of Set A to positive infinitude.
    fn remove_upper_boundes(&mut self);
}
