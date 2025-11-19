pub fn std_module() -> &'static str {
    concat!(
        include_str!("abs.nlang"), "\n",
        include_str!("abs_float.nlang"), "\n",
        include_str!("constants.nlang"), "\n",
        include_str!("basic_math.nlang"), "\n",
        include_str!("sum.nlang"), "\n",
        include_str!("stats.nlang"), "\n",
        include_str!("min.nlang"), "\n",
        include_str!("max.nlang"), "\n",
        include_str!("pow.nlang"), "\n",
        include_str!("exp_ln.nlang"), "\n",
        include_str!("sqrt.nlang"), "\n",
        include_str!("trig.nlang"), "\n",
        include_str!("inverse_trig.nlang"), "\n",
        include_str!("number_theory.nlang"), "\n",
        include_str!("reverse.nlang"), "\n",
        include_str!("sort.nlang"), "\n",
    )
}