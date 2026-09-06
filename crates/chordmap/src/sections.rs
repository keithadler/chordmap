//! Song structure from a bar-level self-similarity matrix: boundaries from
//! checkerboard novelty, groups from diagonal similarity of segments.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    /// Bar index range `[start, end)`.
    pub start: usize,
    pub end: usize,
    /// 0 = "A", 1 = "B", ...
    pub group: usize,
}

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let na = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let nb = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if na < 1e-9 || nb < 1e-9 {
        0.0
    } else {
        dot / (na * nb)
    }
}

/// Segments plus the novelty curve they came from (for debugging).
#[derive(Debug, Clone, PartialEq)]
pub struct Structure {
    pub segments: Vec<Segment>,
    pub novelty: Vec<f32>,
    /// (segment a, segment b, similarity) for every pair.
    pub pair_similarity: Vec<(usize, usize, f32)>,
}

/// `features` is `n_bars * dim`. Returns segments covering all bars.
pub fn segment(features: &[f32], dim: usize, n: usize) -> Structure {
    if n < 8 {
        return Structure {
            segments: vec![Segment {
                start: 0,
                end: n,
                group: 0,
            }],
            novelty: vec![0.0; n],
            pair_similarity: Vec::new(),
        };
    }
    // Centre the features so what is shared by the whole song (its key, its
    // overall sound) stops making every bar look like every other bar.
    let mut centred = features.to_vec();
    for d in 0..dim {
        let mean = (0..n).map(|i| features[i * dim + d]).sum::<f32>() / n as f32;
        for i in 0..n {
            centred[i * dim + d] -= mean;
        }
    }
    let features = &centred;
    let s = |i: usize, j: usize| {
        cosine(
            &features[i * dim..i * dim + dim],
            &features[j * dim..j * dim + dim],
        )
    };
    let mut ssm = vec![0.0f32; n * n];
    for i in 0..n {
        for j in 0..n {
            ssm[i * n + j] = s(i, j);
        }
    }
    // Checkerboard novelty, half-width L bars.
    let l = 4usize.min(n / 4).max(2);
    let taper = |u: i64| (-0.5 * (u as f32 / (l as f32 * 0.6)).powi(2)).exp();
    let mut nov = vec![0.0f32; n];
    for (i, out) in nov.iter_mut().enumerate() {
        let mut acc = 0.0;
        for u in -(l as i64)..(l as i64) {
            for v in -(l as i64)..(l as i64) {
                let (a, b) = (i as i64 + u, i as i64 + v);
                if a < 0 || b < 0 || a >= n as i64 || b >= n as i64 {
                    continue;
                }
                let sign = if (u < 0) == (v < 0) { 1.0 } else { -1.0 };
                acc += sign * taper(u) * taper(v) * ssm[a as usize * n + b as usize];
            }
        }
        *out = acc.max(0.0);
    }
    let max = nov.iter().cloned().fold(0.0f32, f32::max).max(1e-9);
    for v in nov.iter_mut() {
        *v /= max;
    }
    let mean = nov.iter().sum::<f32>() / n as f32;
    let sd = (nov.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / n as f32).sqrt();
    let thresh = (mean + 0.5 * sd).max(0.2);
    let is_peak = |i: usize| {
        i >= 2
            && i + 2 < n
            && nov[i] >= nov[i - 1]
            && nov[i] >= nov[i + 1]
            && nov[i] >= nov[i - 2]
            && nov[i] >= nov[i + 2]
    };
    let mut bounds = vec![0usize];
    let mut i = 2;
    while i + 2 < n {
        if is_peak(i) && nov[i] >= thresh && i - bounds[bounds.len() - 1] >= 4 && n - i >= 4 {
            bounds.push(i);
            i += 4;
        } else {
            i += 1;
        }
    }
    bounds.push(n);
    // Sections longer than 16 bars are rare; split them at their strongest inner peak.
    loop {
        let mut split = None;
        for w in 0..bounds.len() - 1 {
            let (a, b) = (bounds[w], bounds[w + 1]);
            if b - a > 16 {
                let best = (a + 4..b - 3)
                    .filter(|&i| is_peak(i))
                    .max_by(|&x, &y| nov[x].partial_cmp(&nov[y]).unwrap());
                if let Some(i) = best {
                    if nov[i] > 0.05 {
                        split = Some((w + 1, i));
                        break;
                    }
                }
            }
        }
        match split {
            Some((at, i)) => bounds.insert(at, i),
            None => break,
        }
    }
    let mut segs: Vec<Segment> = bounds
        .windows(2)
        .map(|w| Segment {
            start: w[0],
            end: w[1],
            group: usize::MAX,
        })
        .collect();
    // Similarity of two segments: best mean along a diagonal, allowing the
    // repeat to start up to three bars earlier or later than the first time.
    let seg_sim = |a: &Segment, b: &Segment| {
        let mut best = 0.0f32;
        for lag in -3i64..=3 {
            let mut acc = 0.0;
            let mut cnt = 0usize;
            for k in 0..(a.end - a.start) {
                let j = b.start as i64 + k as i64 + lag;
                if j < b.start as i64 || j >= b.end as i64 {
                    continue;
                }
                acc += ssm[(a.start + k) * n + j as usize];
                cnt += 1;
            }
            if cnt >= 4 {
                best = best.max(acc / cnt as f32);
            }
        }
        best
    };
    let m = segs.len();
    let mut parent: Vec<usize> = (0..m).collect();
    fn find(p: &mut Vec<usize>, x: usize) -> usize {
        if p[x] != x {
            let r = find(p, p[x]);
            p[x] = r;
        }
        p[x]
    }
    let mut pairs = Vec::new();
    for a in 0..m {
        for b in a + 1..m {
            pairs.push((seg_sim(&segs[a], &segs[b]), a, b));
        }
    }
    pairs.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
    let pair_similarity: Vec<(usize, usize, f32)> =
        pairs.iter().map(|&(s, a, b)| (a, b, s)).collect();
    // Merge pairs that stand out from this song's own distribution of similarities.
    let merge_at = if pairs.len() >= 3 {
        let mean = pairs.iter().map(|p| p.0).sum::<f32>() / pairs.len() as f32;
        let sd =
            (pairs.iter().map(|p| (p.0 - mean).powi(2)).sum::<f32>() / pairs.len() as f32).sqrt();
        (mean + sd).clamp(0.5, 0.85)
    } else {
        0.7
    };
    for (sim, a, b) in pairs {
        if sim < merge_at {
            break;
        }
        let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
        if ra != rb {
            parent[rb] = ra;
        }
    }
    let mut labels: Vec<usize> = Vec::new();
    for (i, seg) in segs.iter_mut().enumerate() {
        let r = find(&mut parent, i);
        let g = match labels.iter().position(|&x| x == r) {
            Some(p) => p,
            None => {
                labels.push(r);
                labels.len() - 1
            }
        };
        seg.group = g;
    }
    // Neighbours that came out as the same material are one section.
    let mut merged: Vec<Segment> = Vec::with_capacity(segs.len());
    for s in segs {
        match merged.last_mut() {
            Some(last) if last.group == s.group => last.end = s.end,
            _ => merged.push(s),
        }
    }
    // Re-letter in order of first appearance after merging.
    let mut seen: Vec<usize> = Vec::new();
    for s in merged.iter_mut() {
        let g = match seen.iter().position(|&x| x == s.group) {
            Some(p) => p,
            None => {
                seen.push(s.group);
                seen.len() - 1
            }
        };
        s.group = g;
    }
    Structure {
        segments: merged,
        novelty: nov,
        pair_similarity,
    }
}

/// Plain-English guess for each segment given group counts and energies.
pub fn guesses(segs: &[Segment], energy: &[f32]) -> Vec<&'static str> {
    if segs.len() < 2 {
        return vec!["section"; segs.len()];
    }
    let n_groups = segs.iter().map(|s| s.group).max().map_or(0, |g| g + 1);
    let mut count = vec![0usize; n_groups];
    let mut level = vec![0.0f32; n_groups];
    for (i, s) in segs.iter().enumerate() {
        count[s.group] += 1;
        level[s.group] += energy.get(i).copied().unwrap_or(0.0);
    }
    for g in 0..n_groups {
        level[g] /= count[g].max(1) as f32;
    }
    let mut repeated: Vec<usize> = (0..n_groups).filter(|&g| count[g] >= 2).collect();
    repeated.sort_by(|&a, &b| {
        count[b]
            .cmp(&count[a])
            .then(level[b].partial_cmp(&level[a]).unwrap())
    });
    let chorus = repeated
        .iter()
        .copied()
        .max_by(|&a, &b| level[a].partial_cmp(&level[b]).unwrap());
    let verse = repeated.iter().copied().find(|&g| Some(g) != chorus);
    segs.iter()
        .enumerate()
        .map(|(i, s)| {
            if Some(s.group) == chorus {
                "chorus"
            } else if Some(s.group) == verse {
                "verse"
            } else if count[s.group] >= 2 {
                "section"
            } else if i == 0 {
                "intro"
            } else if i + 1 == segs.len() {
                "outro"
            } else {
                "bridge"
            }
        })
        .collect()
}
