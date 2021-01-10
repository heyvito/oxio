pub fn distance(a: &str, b: &str) -> usize {
    let mut res = 0;

    if a == b {
        return res;
    }

    let len_a = a.chars().count();
    let len_b = b.chars().count();

    if len_a == 0 {
        return len_b;
    }

    if len_b == 0 {
        return len_a;
    }

    let mut cache: Vec<usize> = vec![0; len_a];
    let mut idx_a = 0;
    let mut dist_a;
    let mut dist_b;

    while idx_a < len_a {
        idx_a += 1;
        cache[idx_a - 1] = idx_a;
    }


    for (idx_b, code_b) in b.chars().enumerate() {
        res = idx_b;
        dist_a = idx_b;

        for (idx_a, code_a) in a.chars().enumerate() {
            dist_b = if code_a == code_b {
                dist_a
            } else {
                dist_a + 1
            };

            dist_a = cache[idx_a];

            res = if dist_a > res {
                if dist_b > res {
                    res + 1
                } else {
                    dist_b
                }
            } else if dist_b > dist_a {
                dist_a + 1
            } else {
                dist_b
            };

            cache[idx_a] = res;
        }
    }

    res
}