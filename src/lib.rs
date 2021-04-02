use libloading::{Library, Symbol};
use std::{ffi::OsStr, mem::MaybeUninit, os::raw::c_char, sync::Arc};
use std::ffi::CString;
use std::convert::TryFrom;

type Synthe<'a> = Symbol<'a, unsafe extern fn(*const c_char, i32, *mut i32) -> *mut u8>;
type FreeWav<'a> = Symbol<'a, unsafe extern fn(*mut u8)>;

/// # AquesTalk.dllのラッパー
/// 基本的な流れとしてはAquesTalk.dllを読み込む→音声データを生成するというように使います
/// Drop時にAquesTalk_FreeWave()が実行されるため､自分で実行する必要はありません
/// ## Examples
/// ```
/// use testing::AqDLL;
/// use std::{fs::File, io::Write};
///
/// fn main() {
///     let mut reimu = AqDLL::load("./aquestalk/f1/AquesTalk.dll").unwrap();
///     let reimuvoice = reimu.synthe("ゆっくりしていってね", 100).unwrap();
///     let mut file = File::create("./reimu.wav").unwrap();
///     file.write_all(*reimuvoice).unwrap();
/// }
/// ```
pub struct AqDLL<'a>{
    dll: Arc<AqDLL2<'a>>,
}

struct AqDLL2<'a>{
    lib: Library,
    synthe: Synthe<'a>,
    freewav: FreeWav<'a>
}

impl<'a> AqDLL<'a>{
    /// AquesTalk.dllを読み込むための関数です｡引数にはAquesTalk.dllのパスを指定してください
    pub fn load<P: AsRef<OsStr>>(filename: P) -> Result<Self, Box<dyn std::error::Error>>{
        unsafe{
            let aqdll = AqDLL{
                dll: Arc::new(AqDLL2{
                    lib: Library::new(filename)?,
                    synthe: MaybeUninit::uninit().assume_init(),
                    freewav: MaybeUninit::uninit().assume_init(),
                }),
            };
            *(&aqdll.dll.synthe as *const _ as *mut Synthe) = aqdll.dll.lib.get(b"AquesTalk_Synthe_Utf8")?;
            *(&aqdll.dll.freewav as *const _ as *mut FreeWav) = aqdll.dll.lib.get(b"AquesTalk_FreeWave")?;
            Ok(aqdll)
        }
    }

    /// AquesTalk_Synthe_Utf8と同じです｡第一引数は音声記号列､第二引数は発話速度を50-300で指定します
    /// 公式にはもう一つsize引数がありますが､これは内部で使ってるので指定する必要はありません
    pub fn synthe<'b>(&'a self, koe: &str, ispeed: i32) -> Result<AqWav<'b>,Box<dyn std::error::Error>>{
        unsafe{
            let koe2 = CString::new(koe)?;
            let mut size = 0;
            let wav = (self.dll.synthe)(koe2.as_ptr(), ispeed, &mut size as *mut _);
            if wav.is_null(){
                Err(Box::new(AquesTalkErr(size)))
            } else {
                Ok(AqWav{
                    wav: std::slice::from_raw_parts_mut(wav, TryFrom::try_from(size)?),
                    dll: Arc::clone(&*(&self.dll as *const _ as *mut Arc<AqDLL2>)),
                })
            }
        }
    }
}

/// synthe関数で生成されたwavデータへのスマートポインタです
pub struct AqWav<'a>{
    wav: &'a mut [u8],
    dll: Arc<AqDLL2<'a>>,
}

impl<'a> std::ops::Deref for AqWav<'a>{
    type Target = &'a mut [u8];

    fn deref(&self) -> &Self::Target {
        &self.wav
    }
}

impl<'a> std::ops::DerefMut for AqWav<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.wav
    }
}

impl<'a> std::ops::Drop for AqWav<'a> {
    fn drop(&mut self){
        unsafe {
            (self.dll.freewav)(&mut self.wav[0] as *mut _);
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