use std::collections::HashMap;

lazy_static::lazy_static! {
    static ref PINYIN_MAP: HashMap<char, &'static str> = {
        let mut m = HashMap::new();
        
        m.insert('文', "wen"); m.insert('件', "jian"); m.insert('夹', "jia");
        m.insert('资', "zi"); m.insert('源', "yuan"); m.insert('码', "ma");
        m.insert('配', "pei"); m.insert('置', "zhi"); m.insert('设', "she");
        m.insert('搜', "sou"); m.insert('索', "suo"); m.insert('引', "yin");
        m.insert('擎', "qing"); m.insert('数', "shu"); m.insert('据', "ju");
        m.insert('库', "ku"); m.insert('仓', "cang"); m.insert('管', "guan");
        m.insert('理', "li"); m.insert('增', "zeng"); m.insert('删', "shan");
        m.insert('改', "gai"); m.insert('查', "cha"); m.insert('看', "kan");
        m.insert('显', "xian"); m.insert('示', "shi"); m.insert('隐', "yin");
        m.insert('藏', "cang"); m.insert('开', "kai"); m.insert('关', "guan");
        m.insert('启', "qi"); m.insert('停', "ting"); m.insert('运', "yun");
        m.insert('行', "xing"); m.insert('执', "zhi"); m.insert('退', "tui");
        m.insert('登', "deng"); m.insert('录', "lu"); m.insert('注', "zhu");
        m.insert('册', "ce"); m.insert('登', "deng"); m.insert('出', "chu");
        m.insert('入', "ru"); m.insert('进', "jin"); m.insert('出', "chu");
        m.insert('复', "fu"); m.insert('制', "zhi"); m.insert('粘', "zhan");
        m.insert('贴', "tie"); m.insert('剪', "jian"); m.insert('切', "qie");
        m.insert('复', "fu"); m.insert('制', "zhi"); m.insert('粘', "zhan");
        m.insert('贴', "tie"); m.insert('剪', "jian"); m.insert('切', "qie");
        m.insert('全', "quan"); m.insert('选', "xuan"); m.insert('反', "fan");
        m.insert('取', "qu"); m.insert('保', "bao"); m.insert('存', "cun");
        m.insert('删', "shan"); m.insert('除', "chu"); m.insert('清', "qing");
        m.insert('空', "kong"); m.insert('移', "yi"); m.insert('动', "dong");
        m.insert('静', "jing"); m.insert('默', "mo"); m.insert('认', "ren");
        m.insert('自', "zi"); m.insert('动', "dong"); m.insert('手', "shou");
        m.insert('工', "gong"); m.insert('具', "ju"); m.insert('帮', "bang");
        m.insert('助', "zhu"); m.insert('向', "xiang"); m.insert('导', "dao");
        m.insert('引', "yin"); m.insert('导', "dao"); m.insert('出', "chu");
        m.insert('打', "da"); m.insert('印', "yin"); m.insert('预', "yu");
        m.insert('览', "lan"); m.insert('放', "fang"); m.insert('大', "da");
        m.insert('小', "xiao"); m.insert('中', "zhong"); m.insert('上', "shang");
        m.insert('下', "xia"); m.insert('左', "zuo"); m.insert('右', "you");
        m.insert('前', "qian"); m.insert('后', "hou"); m.insert('内', "nei");
        m.insert('外', "wai"); m.insert('里', "li"); m.insert('表', "biao");
        m.insert('格', "ge"); m.insert('式', "shi"); m.insert('样', "yang");
        m.insert('风', "feng"); m.insert('格', "ge"); m.insert('主', "zhu");
        m.insert('题', "ti"); m.insert('目', "mu"); m.insert('标', "biao");
        m.insert('签', "qian"); m.insert('章', "zhang"); m.insert('节', "jie");
        m.insert('页', "ye"); m.insert('面', "mian"); m.insert('版', "ban");
        m.insert('本', "ben"); m.insert('版', "ban"); m.insert('更', "geng");
        m.insert('新', "xin"); m.insert('旧', "jiu"); m.insert('历', "li");
        m.insert('史', "shi"); m.insert('记', "ji"); m.insert('录', "lu");
        m.insert('日', "ri"); m.insert('期', "qi"); m.insert('时', "shi");
        m.insert('间', "jian"); m.insert('地', "di"); m.insert('点', "dian");
        m.insert('秒', "miao"); m.insert('分', "fen"); m.insert('时', "shi");
        m.insert('年', "nian"); m.insert('月', "yue"); m.insert('日', "ri");
        m.insert('周', "zhou"); m.insert('星', "xing"); m.insert('期', "qi");
        m.insert('一', "yi"); m.insert('二', "er"); m.insert('三', "san");
        m.insert('四', "si"); m.insert('五', "wu"); m.insert('六', "liu");
        m.insert('七', "qi"); m.insert('八', "ba"); m.insert('九', "jiu");
        m.insert('十', "shi"); m.insert('百', "bai"); m.insert('千', "qian");
        m.insert('万', "wan"); m.insert('亿', "yi"); m.insert('零', "ling");
        
        m
    };
}

pub fn to_pinyin(text: &str) -> String {
    let mut result = String::new();
    for c in text.chars() {
        if c.is_ascii() {
            result.push(c);
        } else if let Some(&pinyin) = PINYIN_MAP.get(&c) {
            result.push_str(pinyin);
        } else {
            result.push(c);
        }
    }
    result
}

pub fn contains_pinyin(text: &str, query: &str) -> bool {
    let text_pinyin = to_pinyin(text);
    let query_lower = query.to_lowercase();
    
    text.to_lowercase().contains(&query_lower) || text_pinyin.to_lowercase().contains(&query_lower)
}
