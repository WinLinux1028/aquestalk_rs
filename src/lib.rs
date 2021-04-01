use libloading::{Library, Symbol};
use std::{ffi::OsStr, mem::MaybeUninit, os::raw::c_char};
use std::ffi::CString;
use std::convert::TryFrom;

type Synthe<'a> = Symbol<'a, unsafe extern fn(*const c_char, i32, *mut i32) -> *mut u8>;
pub struct AqDLL<'a>{
    lib: Library,
    synthe: Synthe<'a>,
    data: Vec<*mut u8>,
}

impl<'a> AqDLL<'a>{
    pub fn load<P: AsRef<OsStr>>(filename: P) -> Result<Box<Self>, Box<dyn std::error::Error>>{
        unsafe{
            let aqdll = Box::new(AqDLL{
                lib: Library::new(filename)?,
                synthe: MaybeUninit::uninit().assume_init(),
                data: Vec::new(),
            });
            *(&aqdll.synthe as *const _ as *mut Synthe) = aqdll.lib.get(b"AquesTalk_Synthe_Utf8")?;
            Ok(aqdll)
        }
    }

    pub fn synthe(&mut self, koe: &str, ispeed: i32) -> Result<&mut [u8],Box<dyn std::error::Error>>{
        unsafe{
            let koe2 = CString::new(koe)?;
            let mut size = 0;
            let wav = (self.synthe)(koe2.as_ptr(), ispeed, &mut size as *mut _);
            if wav.is_null(){
                Err(Box::new(AquesTalkErr(size)))
            } else {
                self.data.push(wav);
                Ok(std::slice::from_raw_parts_mut(wav, TryFrom::try_from(size)?))
            }   
        }
    }
}

impl<'a> std::ops::Drop for AqDLL<'a> {
    fn drop(&mut self){
        unsafe {
            let freewave: Symbol<unsafe extern fn(*mut u8)> = self.lib.get(b"AquesTalk_FreeWave").unwrap();
            for i in &self.data {
                (freewave)(*i);
            }
        }
    }
}


struct AquesTalkErr(i32);

impl AquesTalkErr{
    fn msg(&self) -> &str{
        match self.0 {
            100 => "その他のエラー, エラーコード: 100",
            101 => "メモリ不足, エラーコード: 101",
            102 => "音声記号列に未定義の読み記号が指定された, エラーコード: 102",
            103 => "韻律データの時間長がマイナスなっている, エラーコード: 103",
            104 => "内部エラー(未定義の区切りコード検出）, エラーコード: 104",
            105 => "音声記号列に未定義の読み記号が指定された, エラーコード: 105",
            106 => "音声記号列のタグの指定が正しくない, エラーコード: 106",
            107 => "タグの長さが制限を越えている（または[>]がみつからない）, エラーコード: 107",
            108 => "タグ内の値の指定が正しくない, エラーコード: 108",
            109 => "WAVE再生ができない（サウンドドライバ関連の問題）, エラーコード: 109",
            110 => "WAVE再生ができない（サウンドドライバ関連の問題非同期再生）, エラーコード: 110",
            111 => "発声すべきデータがない, エラーコード: 111",
            200 => "音声記号列が長すぎる, エラーコード: 200",
            201 => "１つのフレーズ中の読み記号が多すぎる, エラーコード: 201",
            202 => "音声記号列が長い（内部バッファオーバー1）, エラーコード: 202",
            203 => "ヒープメモリ不足, エラーコード: 203",
            204 => "音声記号列が長い（内部バッファオーバー1）, エラーコード: 204",
            _ => "未定義のエラー",
        }
    }
}

impl std::fmt::Display for AquesTalkErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg())
    }
}

impl std::fmt::Debug for AquesTalkErr {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.msg())
    }
}

impl std::error::Error for AquesTalkErr {
    fn description(&self) -> &str {
        self.msg()
    }
}