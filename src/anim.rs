//! The motion-graphics templates. Each one draws a frame at time `t` of a clip lasting `d` seconds.
use crate::gfx::*;

pub struct Tpl { pub id: &'static str, pub name: &'static str, pub dur: f32, pub fields: &'static str, pub clear: bool, pub sample: [&'static str; 6], pub hint: &'static str }
// sample = [tag, num, head, sub, data, ""]
pub const TPLS: &[Tpl] = &[
    Tpl { id: "head", name: "Headline reveal", dur: 5.0, fields: "head", clear: false, sample: ["", "", "The chip that makes your phone *last two days*", "", "", ""], hint: "" },
    Tpl { id: "lower", name: "Lower third", dur: 5.0, fields: "head sub", clear: true, sample: ["", "", "Aisha Bello", "Hardware editor", "", ""], hint: "" },
    Tpl { id: "banner", name: "Breaking banner", dur: 5.0, fields: "tag head", clear: false, sample: ["Breaking", "", "Self-driving taxis are now *live in three new cities*", "", "", ""], hint: "" },
    Tpl { id: "num", name: "Number count-up", dur: 5.0, fields: "num head", clear: false, sample: ["", "4×", "Faster local AI on a laptop", "", "", ""], hint: "" },
    Tpl { id: "sting", name: "Logo sting", dur: 3.5, fields: "", clear: false, sample: ["", "", "", "", "", ""], hint: "" },
    Tpl { id: "trans", name: "Wipe transition", dur: 1.6, fields: "", clear: true, sample: ["", "", "", "", "", ""], hint: "" },
    Tpl { id: "quote", name: "Quote reveal", dur: 6.5, fields: "head sub", clear: false, sample: ["", "", "The best technology disappears. You stop noticing it and start noticing what it lets you do.", "Name Surname, role", "", ""], hint: "" },
    Tpl { id: "ticker", name: "News ticker", dur: 8.0, fields: "tag head", clear: true, sample: ["Live", "", "AI that runs on your phone • Foldables get thinner • A new charging standard arrives", "", "", ""], hint: "Separate headlines with a bullet. It loops seamlessly." },
    Tpl { id: "recap", name: "Weekly recap intro", dur: 5.0, fields: "tag num head", clear: false, sample: ["Weekly recap", "42", "The week in tech, *unwound*", "", "", ""], hint: "" },
    Tpl { id: "chartbar", name: "Bar graph", dur: 6.5, fields: "tag head sub data", clear: false, sample: ["Sample data", "", "Market share by *brand*", "Source: add your source", "Brand A, 34%\nBrand B, 27%\nBrand C, 19%\nBrand D, 12%\nOthers, 8%", ""], hint: "One per line: label, value" },
    Tpl { id: "numcard", name: "Number card", dur: 3.0, fields: "num head", clear: false, sample: ["", "−13%", "Alphabet’s worst day ever", "", "", ""], hint: "" },
    Tpl { id: "daytl", name: "Day timeline", dur: 20.0, fields: "tag head data", clear: false, sample: ["This week", "", "Three days, *one big reveal*", "", "MON|OLED iPad mini|From $699\nTUE|iMac and MacBook Pro|M6 chip\nWED|MacBook Pro with touch|First touch Mac", ""], hint: "One day per line: DAY | title | detail. Up to 5." },
    Tpl { id: "pills", name: "Status pills", dur: 30.0, fields: "tag head data", clear: false, sample: ["Status", "", "What we *know*", "", "REPORTED|Something owners saw\nCONFIRMED|Something the company said\nUNKNOWN|The part nobody can say yet", ""], hint: "One per line: REPORTED, CONFIRMED or UNKNOWN | sentence. Up to 5." },
    Tpl { id: "compare", name: "Comparison rows", dur: 30.0, fields: "tag head data", clear: false, sample: ["The week", "", "Who *fell*", "", "Company A|−13%|Detail line\nCompany B|Missed|Detail line\nCompany C|−29%|Detail line", ""], hint: "One per line: name | big number | detail. Up to 5." },
    Tpl { id: "split", name: "Split bar", dur: 14.0, fields: "tag head num data", clear: false, sample: ["Breakdown", "$80.7B", "Where the loss *came from*", "", "The rest|$8.3B\nNon-cash charge|$72.4B", ""], hint: "Up to 3 lines: label | amount. The largest is lime." },
    Tpl { id: "chains", name: "Chain dots", dur: 14.0, fields: "tag head num data", clear: false, sample: ["Map", "~20 minutes", "Across *ten chains*", "", "Ethereum\nBNB Chain\nTron\nSolana\nPolygon\nArbitrum\nBase\nAvalanche\nOptimism\nBitcoin", ""], hint: "One name per line, up to 12. Dots light in sequence." },
    Tpl { id: "story", name: "Story card", dur: 12.0, fields: "tag head sub", clear: false, sample: ["Source", "", "One *quick* story", "Price or detail", "", ""], hint: "" },
    Tpl { id: "endcard", name: "End card", dur: 10.0, fields: "head data", clear: false, sample: ["", "", "Which story matters *most*?", "", "COMMENT\nSUBSCRIBE", ""], hint: "One pill per line, up to 3." },
];
pub fn tpl(id: &str) -> Option<&'static Tpl> { TPLS.iter().find(|t| t.id == id) }

pub fn draw(c: &mut Cv, p: &Params, t: f32, d: f32) {
    match p.tpl.as_str() {
        "head" => head(c, p, t, d), "lower" => lower(c, p, t, d), "banner" => banner(c, p, t, d), "num" => num(c, p, t, d),
        "sting" => sting(c, p, t, d), "trans" => trans(c, p, t, d), "quote" => quote(c, p, t, d), "ticker" => ticker(c, p, t, d),
        "recap" => recap(c, p, t, d), "chartbar" => chartbar(c, p, t, d), "numcard" => numcard(c, p, t, d), "daytl" => daytl(c, p, t, d),
        "pills" => pills(c, p, t, d), "compare" => compare(c, p, t, d), "split" => split(c, p, t, d), "chains" => chains(c, p, t, d),
        "story" => story(c, p, t, d), "endcard" => endcard(c, p, t, d),
        _ => generic(c, p, t, d),
    }
}

fn chart_head(c: &mut Cv, p: &Params, t: f32, _d: f32, th: &Theme, a: f32, q: f32, ns: bool) -> (f32, f32) {
    let u = c.u;
    let m = 64.0 * u;
    let ph = 64.0 * u;
    c.pill(&p.tag, m, m, ph, th.hl, th.bg, ebk(sg(t, 0.1, 0.6)), eo(sg(t, 0.1, 0.5)) * a);
    let hy = m + ph + 30.0 * u;
    let land = c.w > c.h;
    let bo = c.bo(c.w - 2.0 * m, c.h * if land { 0.26 } else { 0.2 }, if land { 100.0 } else { 84.0 } * u);
    let b = c.block(&p.head, bo);
    let bs = b.s;
    c.lines(&b, m, hy, th.head, th.hl, |i| { let pp = eo(sg(t, 0.3 + i as f32 * 0.14, 1.0 + i as f32 * 0.14)); (1.0, (1.0 - pp) * bs * 1.05 - q * bs * 1.05) }, false);
    let sy = hy + b.th + 14.0 * u;
    if !p.sub.is_empty() && !ns { c.tx(&p.sub, m, sy + 26.0 * u, 26.0 * u, th.sm, TO::default().w(600).a(eo(sg(t, 0.9, 1.4)) * a)); }
    (m, sy + if p.sub.is_empty() { 36.0 } else { 76.0 } * u)
}
fn st8(d: f32, n: usize, k: f32) -> f32 { 0.5f32.max(k.min((d - 4.0) / (n.max(1) as f32))) }
fn generic(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme();
    let (a, q) = (1.0 - sg(t, d - 0.5, d), sg(t, d - 0.7, d - 0.1));
    c.deco(p, Some(c.w * 0.5), Some(c.h * 0.6), 0.0, None);
    chart_head(c, p, t, d, &th, a, q, false);
}

fn head(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h); let m = 64.0 * u; let land = w > h;
    let bo = c.bo(if land { w * 0.74 } else { w - 2.0 * m }, h * 0.6, if land { 150.0 } else { 128.0 } * u);
    let b = c.block(&p.head, bo);
    let y = h - m * 1.7 - b.th; let q = sg(t, d - 0.7, d - 0.1); let bs = b.s;
    c.deco(p, None, None, 0.0, None);
    c.logo_tl(p, sg(t, 0.1, 0.6) * (1.0 - sg(t, d - 0.4, d)));
    c.lines(&b, m, y, th.head, th.hl, |i| { let pp = eo(sg(t, 0.3 + i as f32 * 0.14, 1.0 + i as f32 * 0.14)); (1.0, (1.0 - pp) * bs * 1.05 - q * bs * 1.05) }, false);
    c.fill_rect(m, y - 34.0 * u, 150.0 * u * eo(sg(t, 1.1, 1.7)) * (1.0 - q), 10.0 * u, th.hl);
}

fn lower(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (h, m) = (118.0 * u, 64.0 * u); let (x, y) = (m, c.h - m - h * 1.3);
    let tp = eo(sg(t, 0.1, 0.7)) * (1.0 - eo(sg(t, d - 0.5, d - 0.05)));
    let bp = eo(sg(t, 0.45, 1.05)) * (1.0 - eo(sg(t, d - 0.9, d - 0.4)));
    let (ns, rs) = (44.0 * u, 27.0 * u);
    let name = p.head.to_uppercase();
    let nw = c.measure(&name, 'A', 900, ns, -1.0); let rw = c.measure(&p.sub, 'A', 700, rs, 0.0);
    let bw = nw.max(rw) + 56.0 * u;
    c.save();
    c.clip_rect(x + h, y, bw * bp, h);
    c.fill_rect(x + h, y, bw, h, th.head);
    c.tx(&name, x + h + 28.0 * u, y + h * 0.5, ns, th.bg, TO::default().w(900).ls(-1.0));
    c.tx(&p.sub, x + h + 28.0 * u, y + h * 0.5 + rs * 1.45, rs, th.bg, TO::default().w(700).a(0.7));
    c.restore();
    let tx0 = x - (1.0 - tp) * (h + m + 10.0);
    c.fill_rrect(tx0, y, h, h, 16.0 * u, th.hl);
    c.mark(tx0 + h * 0.2, y + h * 0.2, h * 0.6, None, INK, 1.0, None);
}

fn banner(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h); let m = 64.0 * u; let sh = 104.0 * u; let sy = h * 0.3; let land = w > h;
    let x0 = w * eo(sg(t, d - 0.6, d - 0.1)); let x1 = w * eo(sg(t, 0.2, 0.8));
    let bo = c.bo(if land { w * 0.74 } else { w - 2.0 * m }, h * 0.42, if land { 118.0 } else { 110.0 } * u);
    let b = c.block(&p.head, bo);
    c.deco(p, None, None, 0.0, None);
    c.logo_tl(p, sg(t, 0.1, 0.5) * (1.0 - sg(t, d - 0.4, d)));
    c.save();
    c.clip_rect(x0, sy, (x1 - x0).max(0.0), sh);
    c.fill_rect(0.0, sy, w, sh, th.hl);
    let tag = if p.tag.is_empty() { "BREAKING".to_string() } else { p.tag.to_uppercase() };
    c.tx(&tag, m, sy + sh * 0.66, 50.0 * u, th.bg, TO::default().w(900).ls(-1.0));
    c.restore();
    let q = sg(t, d - 0.7, d - 0.1); let bs = b.s;
    c.lines(&b, m, sy + sh + 44.0 * u, th.head, th.hl, |i| { let pp = eo(sg(t, 0.8 + i as f32 * 0.14, 1.5 + i as f32 * 0.14)); (1.0, (1.0 - pp) * bs * 1.05 - q * bs * 1.05) }, false);
}

/// Splits "$1,234.5M" into prefix, number, suffix like the web editor's regex.
pub fn split_num(s: &str) -> Option<(String, String, String)> {
    let ch: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < ch.len() && !ch[i].is_ascii_digit() && ch[i] != '-' { i += 1; }
    let pre: String = ch[..i].iter().collect();
    let st = i;
    if i < ch.len() && ch[i] == '-' { i += 1; }
    if i >= ch.len() || !ch[i].is_ascii_digit() { return None; }
    while i < ch.len() && (ch[i].is_ascii_digit() || ch[i] == ',') { i += 1; }
    if i < ch.len() && ch[i] == '.' { i += 1; while i < ch.len() && ch[i].is_ascii_digit() { i += 1; } }
    Some((pre, ch[st..i].iter().collect(), ch[i..].iter().collect()))
}
fn num(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h);
    let mut s = if p.num.is_empty() { "0".to_string() } else { p.num.clone() };
    let a = 1.0 - sg(t, d - 0.5, d); let mut txt = s.clone(); let mut sz = 420.0 * u;
    c.deco(p, Some(w / 2.0), Some(h * 0.46), w.min(h) * 0.3 * eo(sg(t, 0.0, 0.8)), None);
    c.logo_tl(p, sg(t, 0.1, 0.5) * a);
    if let Some((pre, n, suf)) = split_num(&s) {
        let v: f32 = n.replace(',', "").parse().unwrap_or(0.0);
        let dec = n.split('.').nth(1).map(|x| x.len()).unwrap_or(0);
        txt = format!("{}{:.*}{}", pre, dec, v * eo(sg(t, 0.3, 2.2)), suf);
        s = format!("{}{:.*}{}", pre, dec, v, suf);
    }
    let w100 = c.measure(&s, 'A', 900, 100.0, -5.0);
    sz = sz.min(100.0 * (w * 0.84) / w100.max(1.0));
    let fw = c.measure(&s, 'A', 900, sz, -0.05 * sz); let cy = h * 0.46;
    c.tx(&txt, w / 2.0 - fw / 2.0, cy + sz * 0.35, sz, th.hl, TO::default().w(900).ls(-0.05 * sz).a(a * eo(sg(t, 0.1, 0.5))));
    c.save(); c.alpha(a);
    c.fill_rect(w / 2.0 - fw / 2.0, cy + sz * 0.5, fw * eo(sg(t, 1.6, 2.4)), 10.0 * u, th.head);
    c.restore();
    let bo = c.bo(w * 0.8, h * 0.2, 84.0 * u);
    let b = c.block(&p.head, bo); let bs = b.s;
    c.lines(&b, w / 2.0, cy + sz * 0.5 + 40.0 * u, th.head, th.hl, |i| { let q = eo(sg(t, 1.9 + i as f32 * 0.14, 2.6 + i as f32 * 0.14)); (a, (1.0 - q) * bs) }, true);
}

fn sting(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h);
    let r = w.min(h) * 0.2; let e = ebk(sg(t, 0.05, 0.75)); let s2 = eio(sg(t, 1.4, 2.2));
    let (cy, cx) = (h / 2.0 - s2 * r * 0.5, w / 2.0); let wo = sg(t, d - 0.6, d - 0.05);
    c.deco(p, Some(cx), Some(cy), r * 1.28 * e, None);
    c.fill_circle(cx, cy, r * e, th.hl);
    c.mark(cx - r * 0.62, cy - r * 0.62 + r * 0.04, r * 1.24, None, INK, eo(sg(t, 0.5, 1.4)), None);
    let sz = (150.0 * u).min(w * 0.8 / 9.2);
    let full = c.measure("TECH UNWIND.", 'A', 900, sz, -0.04 * sz);
    let ty = cy + r + sz * 1.15;
    c.save();
    c.clip_rect(cx - full / 2.0 - 20.0, ty - sz, (full + 40.0) * eo(sg(t, 1.7, 2.5)), sz * 1.3);
    c.tx("TECH UNWIND", cx - full / 2.0, ty, sz, th.head, TO::default().w(900).ls(-0.04 * sz));
    let w1 = c.measure("TECH UNWIND", 'A', 900, sz, -0.04 * sz);
    c.tx(".", cx - full / 2.0 + w1, ty, sz, th.hl, TO::default().w(900).ls(-0.04 * sz));
    c.restore();
    c.fill_rect(-w + w * eo(wo), 0.0, w, h, th.hl);
}

fn trans(c: &mut Cv, p: &Params, t: f32, _d: f32) {
    let th = p.theme(); let (w, h) = (c.w, c.h); let n = 4; let hh = h / n as f32;
    let cols = [th.hl, FOREST, th.hl, FOREST];
    for i in 0..n {
        let f = i as f32;
        let a = eo(sg(t, f * 0.07, f * 0.07 + 0.45)); let b = eo(sg(t, 0.95 + f * 0.07, 0.95 + f * 0.07 + 0.45));
        c.fill_rect(-w + w * a + w * b, f * hh, w, hh + 1.0, cols[i]);
    }
    let pp = ebk(sg(t, 0.5, 0.8)) * (1.0 - sg(t, 0.9, 1.05)); let z = w.min(h) * 0.3 * pp;
    if pp > 0.0 { c.mark(w / 2.0 - z / 2.0, h / 2.0 - z / 2.0, z, Some(INK), th.hl, 1.0, None); }
}

fn quote(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h); let m = 64.0 * u; let a = 1.0 - sg(t, d - 0.5, d); let land = w > h;
    let mut bo = c.bo(if land { w * 0.74 } else { w - 2.0 * m }, h * 0.55, if land { 84.0 } else { 72.0 } * u);
    bo.fam = 'N'; bo.wt = 500; bo.up = false; bo.sp = 0.0; bo.lh = 1.28;
    let b = c.block(&p.head, bo);
    let y = h * 0.5 - b.th * 0.35;
    c.deco(p, None, None, 0.0, None);
    c.logo_tl(p, sg(t, 0.1, 0.5) * a);
    let qp = eo(sg(t, 0.15, 0.8));
    c.tx("“", m - 8.0 * u, y - 30.0 * u, 360.0 * u * (0.8 + 0.2 * qp), th.hl, TO::default().w(900).a(qp * a));
    let mut i = 0.0f32;
    for (li, l) in b.l.iter().enumerate() {
        let mut px = m; let by = y + li as f32 * b.s * b.lh + b.s * 0.82;
        for (j, k) in l.iter().enumerate() {
            if j > 0 && !k.g { px += b.sp; }
            let pp = eo(sg(t, 0.7 + i * 0.15, 1.1 + i * 0.15)); i += 1.0;
            c.tx(&k.w, px, by + (1.0 - pp) * 16.0 * u, b.s, if k.h { th.hl } else { th.head }, TO::default().fam('N').w(500).a(pp * a));
            px += k.tw;
        }
    }
    let e = 0.7 + i * 0.15 + 0.3;
    c.tx(&p.sub, m, y + b.th + 60.0 * u, 30.0 * u, th.hl, TO::default().a(eo(sg(t, e, e + 0.5)) * a));
}

fn ticker(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h); let bh = 96.0 * u;
    let y = h - 64.0 * u * 0.6 - bh + (1.0 - eo(sg(t, 0.0, 0.5))) * bh * 2.0 + eo(sg(t, d - 0.5, d)) * bh * 2.0;
    let tag = if p.tag.is_empty() { "LIVE".to_string() } else { p.tag.to_uppercase() }; let ts = 34.0 * u;
    let tw = c.measure(&tag, 'A', 900, ts, -1.0); let cap = bh + tw + 44.0 * u;
    c.fill_rect(0.0, y, w, bh, th.hl);
    let base = format!("{}     •     ", if p.head.is_empty() { "Tech Unwind" } else { &p.head });
    let mut unit = base.clone();
    while c.measure(&unit, 'A', 800, 36.0 * u, 0.0) < w * 0.7 { unit.push_str(&base); }
    let uw = c.measure(&unit, 'A', 800, 36.0 * u, 0.0); let off = (t / d) * uw;
    c.save();
    c.clip_rect(cap, y, w - cap, bh);
    let mut k = -1;
    while (k as f32) < (w / uw).ceil() + 2.0 { c.tx(&unit, cap + k as f32 * uw - off, y + bh * 0.64, 36.0 * u, th.bg, TO::default().w(800)); k += 1; }
    c.restore();
    c.fill_rect(0.0, y, cap, bh, th.bg);
    c.mark(bh * 0.2, y + bh * 0.2, bh * 0.6, None, th.hl, 1.0, None);
    c.tx(&tag, bh + 8.0 * u, y + bh * 0.64, ts, th.hl, TO::default().w(900).ls(-1.0));
}

fn recap(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h);
    let r = w.min(h) * 0.2; let cy = h * 0.4; let a = 1.0 - sg(t, d - 0.5, d); let e = ebk(sg(t, 0.2, 0.9));
    c.deco(p, Some(w / 2.0), Some(cy), r * 1.5 * e, None);
    c.logo_tl(p, sg(t, 0.1, 0.5) * a);
    c.save(); c.alpha(a);
    c.fill_circle(w / 2.0, cy, r * e, th.head);
    c.stroke_circle(w / 2.0, cy, (r + 28.0 * u) * e, th.hl, 1.0, 6.0 * u, Some((vec![16.0 * u, 16.0 * u], -t * 40.0 * u)));
    c.restore();
    c.tx("EP.", w / 2.0, cy - r * 0.5, r * 0.17, th.bg, TO::default().al(Al::Center).a(a * eo(sg(t, 0.7, 1.1))));
    let mut nz = r * 0.95;
    while c.measure(&p.num, 'A', 900, nz, -0.05 * nz) > r * 1.5 && nz > 20.0 { nz -= 4.0; }
    c.tx(&p.num, w / 2.0, cy + nz * 0.4, nz, th.bg, TO::default().w(900).ls(-0.05 * nz).al(Al::Center).a(a * eo(sg(t, 0.8, 1.3))));
    c.tx(&p.tag.to_uppercase(), w / 2.0, cy - r * 1.5 - 26.0 * u, 36.0 * u, th.hl, TO::default().al(Al::Center).a(a * eo(sg(t, 0.9, 1.4))));
    let bo = c.bo(w * 0.84, h * 0.22, 100.0 * u);
    let b = c.block(&p.head, bo); let bs = b.s;
    c.lines(&b, w / 2.0, cy + r + 70.0 * u, th.head, th.hl, |i| { let q = eo(sg(t, 1.2 + i as f32 * 0.14, 1.9 + i as f32 * 0.14)); (a, (1.0 - q) * bs) }, true);
}

fn numcard(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h);
    let a = 1.0 - sg(t, d - 0.4, d); let pp = eo(sg(t, 0.0, 0.17));
    let sz = c.fz(&p.num, 400.0 * u, w * 0.86, 900); let cy = h * 0.44;
    c.deco(p, Some(w / 2.0), Some(cy), 0.0, None);
    c.logo_tl(p, a);
    let tw = c.measure(&p.num, 'A', 900, sz, -0.05 * sz);
    c.save(); c.translate(w / 2.0, cy); c.scale(0.95 + 0.05 * pp, 0.95 + 0.05 * pp);
    c.tx(&p.num, -tw / 2.0, sz * 0.35, sz, th.hl, TO::default().w(900).ls(-0.05 * sz).a(a * pp));
    c.restore();
    let bo = c.bo(w * 0.8, h * 0.2, 60.0 * u);
    let b = c.block(&p.head, bo); let bs = b.s;
    c.lines(&b, w / 2.0, cy + sz * 0.5 + 30.0 * u, th.head, th.hl, |i| { let q = eo(sg(t, 0.25 + i as f32 * 0.15, 0.9 + i as f32 * 0.15)); (a, (1.0 - q) * bs) }, true);
}

// ----- data helpers -----
struct Row { l: String, v: f32, pre: String, suf: String, dec: usize }
fn parse_data(p: &Params) -> Vec<Row> {
    p.data.split('\n').filter_map(|s| {
        let i = s.rfind(',')?;
        let l = s[..i].trim().to_string();
        let (pre, n, suf) = split_num(s[i + 1..].trim())?;
        if l.is_empty() { return None; }
        let dec = n.split('.').nth(1).map(|x| x.len()).unwrap_or(0);
        Some(Row { l, v: n.replace(',', "").parse().ok()?, pre, suf, dec })
    }).take(12).collect()
}
fn fmtv(r: &Row, v: f32) -> String {
    let s = format!("{:.*}", r.dec, v);
    let (int, frac) = match s.split_once('.') { Some((a, b)) => (a.to_string(), Some(b.to_string())), None => (s.clone(), None) };
    let neg = int.starts_with('-'); let digits: Vec<char> = int.trim_start_matches('-').chars().collect();
    let mut out = String::new();
    for (i, ch) in digits.iter().enumerate() { if i > 0 && (digits.len() - i) % 3 == 0 { out.push(','); } out.push(*ch); }
    format!("{}{}{}{}{}", r.pre, if neg { "-" } else { "" }, out, frac.map(|f| format!(".{}", f)).unwrap_or_default(), r.suf)
}
fn nice(v: f32) -> f32 {
    let v = if v == 0.0 { 1.0 } else { v };
    let pw = 10f32.powf(v.log10().floor()); let f = v / pw;
    (if f <= 1.0 { 1.0 } else if f <= 2.0 { 2.0 } else if f <= 2.5 { 2.5 } else if f <= 5.0 { 5.0 } else { 10.0 }) * pw
}

fn chartbar(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h); let r = parse_data(p);
    let (a, q, n) = (1.0 - sg(t, d - 0.5, d), sg(t, d - 0.7, d - 0.1), r.len());
    c.deco(p, Some(w * 0.7), Some(h * 0.6), 0.0, None);
    let (m, top) = chart_head(c, p, t, d, &th, a, q, false);
    if n < 2 { c.tx("Add at least two rows of data", w / 2.0, h / 2.0, 36.0 * u, th.sm, TO::default().al(Al::Center)); return; }
    let (l, rr, tp, bt) = (m, w - m, top + 50.0 * u, h - m - 70.0 * u);
    let sl = (rr - l) / n as f32; let bw = sl * 0.62;
    let mv = r.iter().map(|x| x.v).fold(f32::MIN, f32::max); let mx = nice(mv);
    let im = r.iter().position(|x| x.v == mv).unwrap_or(0);
    let mut pb = tiny_skia::PathBuilder::new(); pb.move_to(l, bt); pb.line_to(rr, bt);
    if let Some(path) = pb.finish() { c.stroke_path(&path, th.head, 0.25 * a * eo(sg(t, 0.2, 0.8)), 3.0 * u, tiny_skia::LineCap::Butt, tiny_skia::LineJoin::Miter, None); }
    for (i, row) in r.iter().enumerate() {
        let f = i as f32;
        let pp = eo(sg(t, 0.7 + f * 0.12, 1.7 + f * 0.12)); let hh = (bt - tp) * row.v / mx * pp; let x0 = l + sl * f + (sl - bw) / 2.0;
        c.fill_rrect4(x0, bt - hh, bw, hh.max(0.0), [12.0 * u, 12.0 * u, 0.0, 0.0], if i == im { th.hl } else { th.head }, a * if i == im { 1.0 } else { 0.3 });
        let fs = (40.0 * u).min(bw * 0.34);
        c.tx(&fmtv(row, row.v * pp), x0 + bw / 2.0, bt - hh - 18.0 * u, fs, if i == im { th.hl } else { th.head }, TO::default().al(Al::Center).w(900).ls(-1.0).a(a * sg(pp, 0.1, 0.5)));
        c.tx(&row.l, x0 + bw / 2.0, bt + 48.0 * u, (26.0 * u).min(sl * 0.3), th.sm, TO::default().al(Al::Center).w(600).a(a * eo(sg(t, 0.6 + f * 0.12, 1.1 + f * 0.12))));
    }
}

fn daytl(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h);
    let rows: Vec<_> = p.rows().into_iter().take(5).collect(); let n = rows.len().max(1);
    let (a, q) = (1.0 - sg(t, d - 0.5, d), sg(t, d - 0.7, d - 0.1));
    c.deco(p, Some(w * 0.5), Some(h * 0.6), 0.0, None);
    let (m, top) = chart_head(c, p, t, d, &th, a, q, true);
    let (l, rr) = (m + 10.0 * u, w - m - 10.0 * u); let cw = (rr - l) / n as f32; let y = top + (h - top - m) * 0.3; let sp = st8(d, n, 6.0);
    c.save(); c.alpha(a);
    c.fill_rect_a(l, y, rr - l, 4.0 * u, th.head, 0.25);
    c.fill_rect(l, y, eo(sg(t, 0.6, 0.6 + sp * n as f32)) * (rr - l), 4.0 * u, th.hl);
    c.restore();
    for (i, r) in rows.iter().enumerate() {
        let t0 = 0.6 + i as f32 * sp; let pp = eo(sg(t, t0, t0 + 0.6)); let cx = l + cw * i as f32; let dy = (1.0 - pp) * 20.0 * u;
        if pp <= 0.0 { continue; }
        c.save(); c.alpha(a * pp); c.fill_circle(cx + 14.0 * u, y + 2.0 * u, 14.0 * u * pp, th.hl); c.restore();
        let z0 = c.fz(cell(r, 0), 90.0 * u, cw * 0.9, 900);
        c.tx(cell(r, 0), cx, y - 48.0 * u + dy, z0, th.head, TO::default().w(900).ls(-2.0).a(a * pp));
        let s1 = c.fz(cell(r, 1), 44.0 * u, cw - 24.0 * u, 800);
        c.tx(cell(r, 1), cx, y + 84.0 * u + dy, s1, th.head, TO::default().w(800).a(a * pp));
        let z2 = c.fz(cell(r, 2), 34.0 * u, cw - 24.0 * u, 600);
        c.tx(cell(r, 2), cx, y + 84.0 * u + s1 * 1.3 + dy, z2, th.sm, TO::default().w(600).a(a * pp));
    }
}

fn pills(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h);
    let rows: Vec<_> = p.rows().into_iter().take(5).collect(); let n = rows.len().max(1);
    let (a, q) = (1.0 - sg(t, d - 0.5, d), sg(t, d - 0.7, d - 0.1));
    c.deco(p, Some(w * 0.5), Some(h * 0.6), 0.0, None);
    let (m, top) = chart_head(c, p, t, d, &th, a, q, true);
    let rh = (260.0 * u).min((h - top - m) / n as f32); let sp = st8(d, n, 7.0); let ph = 64.0 * u;
    for (i, r) in rows.iter().enumerate() {
        let t0 = 0.6 + i as f32 * sp; let y = top + i as f32 * rh; let k = cell(r, 0).to_uppercase();
        let f = if k == "CONFIRMED" { th.hl } else if k == "REPORTED" { th.head } else { MOSS };
        let g = if k == "UNKNOWN" { th.head } else { INK };
        c.pill(&k, m, y, ph, f, g, ebk(sg(t, t0, t0 + 0.5)), a * eo(sg(t, t0, t0 + 0.3)));
        let mut bo = c.bo(w - 2.0 * m, rh - ph - 30.0 * u, 52.0 * u);
        bo.s1 = 28.0 * u; bo.wt = 600; bo.sp = 0.0; bo.up = false; bo.lh = 1.1;
        let b = c.block(cell(r, 1), bo); let bs = b.s;
        c.lines(&b, m, y + ph + 16.0 * u, th.head, th.hl, |_| { let pp = eo(sg(t, t0 + 0.25, t0 + 0.85)); (a * pp, (1.0 - pp) * bs) }, false);
    }
}

fn compare(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h);
    let rows: Vec<_> = p.rows().into_iter().take(5).collect(); let n = rows.len().max(1);
    let (a, q) = (1.0 - sg(t, d - 0.5, d), sg(t, d - 0.7, d - 0.1));
    c.deco(p, Some(w * 0.6), Some(h * 0.6), 0.0, None);
    let (m, top) = chart_head(c, p, t, d, &th, a, q, true);
    let rh = (170.0 * u).min((h - top - m) / n as f32); let sp = st8(d, n, 5.0);
    for (i, r) in rows.iter().enumerate() {
        let t0 = 0.7 + i as f32 * sp; let pp = eo(sg(t, t0, t0 + 0.6)); let y = top + i as f32 * rh; let dy = (1.0 - pp) * 24.0 * u;
        c.fill_rect_a(m, y + rh, (w - 2.0 * m) * pp, 2.0 * u, th.head, 0.3 * a * pp);
        let name = cell(r, 0).to_uppercase();
        let z = c.fz(&name, rh * 0.4, w * 0.4, 900);
        c.tx(&name, m, y + rh * 0.5 + dy, z, th.head, TO::default().w(900).ls(-2.0).a(a * pp));
        let z2 = c.fz(cell(r, 2), rh * 0.2, w * 0.45, 600);
        c.tx(cell(r, 2), m, y + rh * 0.78 + dy, z2, th.sm, TO::default().w(600).a(a * pp));
        let bz = c.fz(cell(r, 1), rh * 0.7, w * 0.42, 900);
        let bw = c.measure(cell(r, 1), 'A', 900, bz, -0.04 * bz);
        c.save(); c.translate(w - m, y + rh * 0.5); c.scale(0.95 + 0.05 * pp, 0.95 + 0.05 * pp);
        c.tx(cell(r, 1), -bw, bz * 0.35, bz, th.hl, TO::default().w(900).ls(-0.04 * bz).a(a * pp));
        c.restore();
    }
}

fn split(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h);
    let rows: Vec<_> = p.rows().into_iter().take(3).collect();
    let (a, q) = (1.0 - sg(t, d - 0.5, d), sg(t, d - 0.7, d - 0.1));
    c.deco(p, Some(w * 0.5), Some(h * 0.6), 0.0, None);
    let (m, top) = chart_head(c, p, t, d, &th, a, q, true);
    let vals: Vec<f32> = rows.iter().map(|r| cell(r, 1).chars().filter(|ch| ch.is_ascii_digit() || *ch == '.').collect::<String>().parse().unwrap_or(0.0)).collect();
    let tot = { let s: f32 = vals.iter().sum(); if s == 0.0 { 1.0 } else { s } };
    let mx = vals.iter().cloned().fold(f32::MIN, f32::max); let big = vals.iter().position(|v| *v == mx).unwrap_or(0);
    let bh = (200.0 * u).min(h * 0.2); let by = top + (h - top - m) * 0.4; let e = eo(sg(t, 0.3, 1.1));
    let gp = 22.0 * u * eo(sg(t, 1.6, 2.2)); let aw = w - 2.0 * m - gp * (rows.len().max(1) - 1) as f32; let mut x = m;
    if !p.num.is_empty() {
        let nw = c.measure(&p.num, 'A', 900, 120.0 * u, -4.0);
        c.tx(&p.num, w - m - nw, top + 60.0 * u, 120.0 * u, th.head, TO::default().w(900).ls(-4.0).a(a * e));
    }
    for (i, r) in rows.iter().enumerate() {
        let f = i as f32;
        let bw = aw * vals[i] / tot * e; let la = eo(sg(t, 2.2 + f * 0.5, 2.8 + f * 0.5)); let hot = i == big;
        c.fill_rrect4(x, by, bw.max(1.0), bh, [16.0 * u; 4], if hot { th.hl } else { th.head }, a * if hot { 1.0 } else { 0.85 });
        let nr = bw < 340.0 * u;
        let vy = if nr { by - 70.0 * u } else { by + bh + 70.0 * u }; let ny = if nr { by - 28.0 * u } else { by + bh + 112.0 * u };
        let dl = (1.0 - la) * 20.0 * u * if nr { -1.0 } else { 1.0 };
        let z = c.fz(cell(r, 1), 64.0 * u, bw.max(220.0 * u), 900);
        c.tx(cell(r, 1), x, vy + dl, z, if hot { th.hl } else { th.head }, TO::default().w(900).ls(-1.0).a(a * la));
        let z2 = c.fz(cell(r, 0), 32.0 * u, bw.max(300.0 * u), 600);
        c.tx(cell(r, 0), x, ny + dl, z2, th.sm, TO::default().w(600).a(a * la));
        x += bw + gp;
    }
}

fn chains(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h);
    let rows: Vec<_> = p.rows().into_iter().take(12).collect(); let n = rows.len().max(1);
    let (a, q) = (1.0 - sg(t, d - 0.5, d), sg(t, d - 0.7, d - 0.1));
    c.deco(p, Some(w * 0.5), Some(h * 0.6), 0.0, None);
    let (m, top) = chart_head(c, p, t, d, &th, a, q, false);
    let cols = (n as f32 / 2.0).ceil() as usize; let cw = (w - 2.0 * m) / cols as f32; let rr = (cw * 0.22).min(70.0 * u); let y0 = top + (h - top - m) * 0.2;
    for (i, r) in rows.iter().enumerate() {
        let t0 = 0.6 + i as f32 * 0.32; let pp = eo(sg(t, t0, t0 + 0.4)); let pu = sg(t, t0, t0 + 0.9);
        let cx = m + cw * (i % cols) as f32 + cw / 2.0; let cy = y0 + (i / cols) as f32 * rr * 3.4;
        c.stroke_circle(cx, cy, rr, th.head, 0.3 * a, 4.0 * u, None);
        c.save(); c.alpha(a); c.fill_circle(cx, cy, rr * pp, th.hl); c.restore();
        if pu > 0.0 && pu < 1.0 { c.stroke_circle(cx, cy, rr * (1.0 + pu * 0.9), th.hl, a * (1.0 - pu), 4.0 * u, None); }
        let z = c.fz(cell(r, 0), 30.0 * u, cw * 0.9, 700);
        c.tx(cell(r, 0), cx, cy + rr + 44.0 * u, z, th.sm, TO::default().al(Al::Center).w(700).a(a * pp));
    }
    c.tx(&p.num, w - m, h - m * 0.7, 56.0 * u, th.head, TO::default().al(Al::Right).w(900).ls(-1.0).a(a * eo(sg(t, 0.6 + n as f32 * 0.32, 1.4 + n as f32 * 0.32))));
}

fn story(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h);
    let a = eo(sg(t, 0.0, 0.5)) * (1.0 - eo(sg(t, d - 0.45, d)));
    let sx = (1.0 - eo(sg(t, 0.0, 0.6))) * w * 0.5 - eo(sg(t, d - 0.5, d)) * w * 0.3;
    let m = 64.0 * u; let (cw, ch) = (w - 2.0 * m, h - 2.0 * m);
    c.deco(p, Some(w * 0.5), Some(h * 0.5), 0.0, None);
    c.save(); c.translate(sx, 0.0); c.alpha(a);
    c.fill_rrect(m, m, cw, ch, 28.0 * u, th.head);
    c.pill(&p.tag, m + 60.0 * u, m + 60.0 * u, 72.0 * u, INK, th.hl, 1.0, a);
    let mut bo = c.bo(cw - 120.0 * u, ch * 0.5, 150.0 * u); bo.s1 = 44.0 * u;
    let b = c.block(&p.head, bo); let bs = b.s;
    c.lines(&b, m + 60.0 * u, m + ch * 0.28, INK, MOSS, |i| { let pp = eo(sg(t, 0.3 + i as f32 * 0.14, 0.9 + i as f32 * 0.14)); (a * pp, (1.0 - pp) * bs * 0.6) }, false);
    c.tx(&p.sub, m + 60.0 * u, m + ch - 70.0 * u, 40.0 * u, Col(0x4f, 0x63, 0x57), TO::default().w(700).a(a * eo(sg(t, 0.9, 1.4))));
    c.restore();
}

fn endcard(c: &mut Cv, p: &Params, t: f32, d: f32) {
    let th = p.theme(); let u = c.u; let (w, h) = (c.w, c.h);
    let (a, q) = (1.0 - sg(t, d - 0.5, d), sg(t, d - 0.7, d - 0.1));
    let m = 64.0 * u; let r = w.min(h) * 0.2; let e = ebk(sg(t, 0.05, 0.75)); let (cx, cy) = (w * 0.76, h * 0.46);
    c.deco(p, Some(cx), Some(cy), r * 1.3 * e, None);
    c.save(); c.alpha(a); c.fill_circle(cx, cy, r * e, th.hl); c.restore();
    c.mark(cx - r * 0.62, cy - r * 0.62, r * 1.24, None, INK, eo(sg(t, 0.5, 1.4)), Some(a));
    c.stamp_rings(cx, cy, r, t, a, th.hl, e);
    let mut bo = c.bo(w * 0.5, h * 0.5, 130.0 * u); bo.s1 = 44.0 * u;
    let b = c.block(&p.head, bo); let bs = b.s; let y = h * 0.4 - b.th / 2.0;
    c.lines(&b, m, y, th.head, th.hl, |i| { let pp = eo(sg(t, 0.3 + i as f32 * 0.15, 1.0 + i as f32 * 0.15)); (a, (1.0 - pp) * bs - q * bs) }, false);
    for (i, r) in p.rows().iter().take(3).enumerate() {
        let f = i as f32;
        c.pill(cell(r, 0), m, y + b.th + 50.0 * u + f * 96.0 * u, 72.0 * u, if i > 0 { th.head } else { MOSS }, if i > 0 { INK } else { th.head }, ebk(sg(t, 1.1 + f * 0.3, 1.6 + f * 0.3)), a);
    }
}
