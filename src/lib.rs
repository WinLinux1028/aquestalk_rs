/// # AquesTalk1のラッパー
/// 基本的な流れとしてはAquesTalk.dllを読み込む→音声データを生成するというように使います
/// ## Examples
/// ```
/// use testing::aquestalk1::AqDLL;
/// use std::{fs::File, io::Write};
///
/// fn main() {
///     let reimu = AqDLL::load("./aquestalk/f1/AquesTalk.dll").unwrap();
///     let reimuvoice = reimu.synthe("ゆっくりしていってね", 100).unwrap();
///     let mut file = File::create("./reimu.wav").unwrap();
///     file.write_all(*reimuvoice).unwrap();
/// }
/// ```
#[allow(clippy::needless_doctest_main)]
pub mod aquestalk1 {
    use libloading::{Library, Symbol};
    use safety_breaker::{force_convert, ForceMut};
    use std::{
        convert::TryFrom,
        ffi::{CString, OsStr},
        mem::MaybeUninit,
        os::raw::c_char,
        sync::Arc,
    };
    type AqSynthe<'a> = Symbol<'a, unsafe extern "C" fn(*const c_char, i32, *mut i32) -> *mut u8>;
    type AqFreeWav<'a> = Symbol<'a, unsafe extern "C" fn(*mut u8)>;

    /// DLL内の関数にアクセスするためのラッパー
    pub struct AqDLL<'a> {
        dll: Arc<AqDLL2<'a>>,
    }

    struct AqDLL2<'a> {
        lib: Library,
        synthe: AqSynthe<'a>,
        freewav: AqFreeWav<'a>,
    }

    impl<'a> AqDLL<'a> {
        /// AquesTalk.dllを読み込むための関数です｡引数にはAquesTalk.dllのパスを指定してください
        #[allow(clippy::uninit_assumed_init)]
        pub fn load<P: AsRef<OsStr>>(dllpath: P) -> Result<Self, Box<dyn std::error::Error>> {
            unsafe {
                let dll = AqDLL {
                    dll: Arc::new(AqDLL2 {
                        lib: Library::new(dllpath)?,
                        synthe: MaybeUninit::uninit().assume_init(),
                        freewav: MaybeUninit::uninit().assume_init(),
                    }),
                };
                *dll.dll.synthe.forcemut() = dll.dll.lib.get(b"AquesTalk_Synthe_Utf8")?;
                *dll.dll.freewav.forcemut() = dll.dll.lib.get(b"AquesTalk_FreeWave")?;
                Ok(dll)
            }
        }

        /// AquesTalk_Synthe_Utf8と同じです｡第一引数は音声記号列､第二引数は発話速度を50-300で指定します
        pub fn synthe<'b>(
            &self,
            koe: &str,
            ispeed: i32,
        ) -> Result<AqWAV<'b>, Box<dyn std::error::Error>> {
            unsafe {
                let koe2 = CString::new(koe)?;
                let mut size = 0;
                let wav = (self.dll.synthe)(koe2.as_ptr(), ispeed, &mut size as *mut i32);
                if wav.is_null() {
                    Err(Box::new(AqErr(size)))
                } else {
                    Ok(AqWAV {
                        wav: std::slice::from_raw_parts_mut(wav, TryFrom::try_from(size)?),
                        // dll: Arc::clone(&*(&self.dll as *const _ as *mut Arc<AqDLL2>)),
                        dll: Arc::clone(force_convert!(&self.dll, Arc<AqDLL2>)),
                    })
                }
            }
        }
    }

    /// # synthe関数で生成されたwavデータへのスマートポインタ
    /// このスマートポインタを参照外しするとWAVデータのスライスが出てきます
    /// AquesTalk_FreeWaveはDrop時に実行されるため､自分で実行する必要はありません
    pub struct AqWAV<'a> {
        wav: &'a mut [u8],
        dll: Arc<AqDLL2<'a>>,
    }

    impl<'a> std::ops::Deref for AqWAV<'a> {
        type Target = &'a mut [u8];

        fn deref(&self) -> &Self::Target {
            &self.wav
        }
    }

    impl<'a> std::ops::DerefMut for AqWAV<'a> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.wav
        }
    }

    impl<'a> std::ops::Drop for AqWAV<'a> {
        fn drop(&mut self) {
            unsafe {
                (self.dll.freewav)(&mut self.wav[0] as *mut u8);
            }
        }
    }

    struct AqErr(i32);

    impl AqErr {
        fn msg(&self) -> &str {
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
                110 => {
                    "WAVE再生ができない（サウンドドライバ関連の問題非同期再生）, エラーコード: 110"
                }
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

    impl std::fmt::Display for AqErr {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}", self.msg())
        }
    }

    impl std::fmt::Debug for AqErr {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}", self.msg())
        }
    }

    impl std::error::Error for AqErr {
        fn description(&self) -> &str {
            self.msg()
        }
    }
}

/// # AqKanji2Koeのラッパー
/// 基本的な流れとしてはAqKanji2Koe.dllを読み込む→インスタンスを生成する→漢字かな混じりのテキストを音声記号列に変換するというように使います
/// ## Examples
/// ```
/// use aquestalk_rs::aqkanji2koe::AqK2KDLL;
///
/// fn main() {
///     let aqk2k = AqK2KDLL::load("./aqk2k/lib64/AqKanji2Koe.dll", None).unwrap();
///     let mut aqk2kins = aqk2k.create("./aqk2k/aq_dic").unwrap();
///     let word = aqk2kins.convert("ゆっくりしていってね", None).unwrap();
///     println!("{}", *word);
/// }
/// ```
#[allow(clippy::needless_doctest_main)]
pub mod aqkanji2koe {
    use libloading::{Library, Symbol};
    use safety_breaker::{force_convert, ForceMut};
    use std::{
        alloc,
        convert::TryFrom,
        ffi::{c_void, CStr, CString, OsStr},
        mem,
        mem::MaybeUninit,
        os::raw::c_char,
        sync::Arc,
    };
    type AqK2Kcreate<'a> = Symbol<'a, unsafe extern "C" fn(*const c_char, *mut i32) -> *mut c_void>;
    type AqK2Kcreateptr<'a> =
        Symbol<'a, unsafe extern "C" fn(*const c_void, *const c_void, *mut i32) -> *mut c_void>;
    type AqK2Krelease<'a> = Symbol<'a, unsafe extern "C" fn(*mut c_void)>;
    type AqK2Ksetdevkey<'a> = Symbol<'a, unsafe extern "C" fn(*const c_char) -> i32>;
    type AqK2Kconvert<'a> =
        Symbol<'a, unsafe extern "C" fn(*mut c_void, *const c_char, *mut c_char, i32) -> i32>;

    /// # DLL内の基本的な関数にアクセスするためのラッパー
    pub struct AqK2KDLL<'a> {
        dll: Arc<AqK2KDLL2<'a>>,
    }

    struct AqK2KDLL2<'a> {
        #[allow(dead_code)]
        cpp: Option<Library>,
        lib: Library,
        create: AqK2Kcreate<'a>,
        create_ptr: AqK2Kcreateptr<'a>,
        release: AqK2Krelease<'a>,
        convert: AqK2Kconvert<'a>,
        setdevkey: AqK2Ksetdevkey<'a>,
    }

    impl<'a> AqK2KDLL<'a> {
        /// 第一引数にはAqKanji2Koe.dllのパスを､第二引数には開発ライセンスキーを持っていればSome("(ライセンスキー)")を､持っていなければNoneを指定してください
        /// なお､この制限解除機能は私は製品版を持ってなくてテストしていないので､動作保証はありません(不具合があったら私に製品版をプレゼントするなり､Githubにプルリク投げるなりしてください)
        #[allow(clippy::uninit_assumed_init)]
        pub fn load<P: AsRef<OsStr>>(
            dllpath: P,
            devkey: Option<&str>,
        ) -> Result<Self, Box<dyn std::error::Error>> {
            unsafe {
                let libcpp = Self::cpp()?;
                let dll = AqK2KDLL {
                    dll: Arc::new(AqK2KDLL2 {
                        cpp: libcpp,
                        lib: Library::new(dllpath)?,
                        create: MaybeUninit::uninit().assume_init(),
                        create_ptr: MaybeUninit::uninit().assume_init(),
                        release: MaybeUninit::uninit().assume_init(),
                        convert: MaybeUninit::uninit().assume_init(),
                        setdevkey: MaybeUninit::uninit().assume_init(),
                    }),
                };
                *dll.dll.setdevkey.forcemut() = dll.dll.lib.get(b"AqKanji2Koe_SetDevKey")?;
                if let Some(s) = devkey {
                    let s2 = CString::new(s)?;
                    let _ = (dll.dll.setdevkey)(s2.as_ptr());
                }
                *dll.dll.create.forcemut() = dll.dll.lib.get(b"AqKanji2Koe_Create")?;
                *dll.dll.create_ptr.forcemut() = dll.dll.lib.get(b"AqKanji2Koe_Create_Ptr")?;
                *dll.dll.release.forcemut() = dll.dll.lib.get(b"AqKanji2Koe_Release")?;
                *dll.dll.convert.forcemut() = dll.dll.lib.get(Self::conv())?;
                Ok(dll)
            }
        }

        #[cfg(target_os = "linux")]
        fn cpp() -> Result<Option<Library>, Box<dyn std::error::Error>> {
            unsafe {
                Ok(Some(Library::from(libloading::os::unix::Library::open(
                    Some("libstdc++.so.6"),
                    libloading::os::unix::RTLD_LAZY | libloading::os::unix::RTLD_GLOBAL,
                )?)))
            }
        }

        #[cfg(not(target_os = "linux"))]
        fn cpp() -> Result<Option<Library>, Box<dyn std::error::Error>> {
            Ok(None)
        }

        #[cfg(target_os = "windows")]
        fn conv() -> &'static [u8] {
            b"AqKanji2Koe_Convert_utf8"
        }

        #[cfg(not(target_os = "windows"))]
        fn conv() -> &'static [u8] {
            b"AqKanji2Koe_Convert"
        }

        /// 本家のAqKanji2Koe_Createに当たります
        /// 引数には辞書のあるディレクトリを指定してください
        pub fn create<'b>(
            &self,
            pathdic: &str,
        ) -> Result<AqK2Kinstance<'b>, Box<dyn std::error::Error>> {
            let mut errcode: i32 = 0;
            let pathdic2 = CString::new(pathdic)?;
            unsafe {
                let instance = (self.dll.create)(pathdic2.as_ptr(), &mut errcode as *mut i32);
                if instance.is_null() {
                    Err(Box::new(AqK2Kerr(errcode)))
                } else {
                    Ok(AqK2Kinstance {
                        instance,
                        dll: Arc::clone(force_convert!(&self.dll, Arc<AqK2KDLL2>)),
                    })
                }
            }
        }

        /// 本家のAqKanji2Koe_Create_Ptrに当たります
        /// 第一引数にはシステム辞書の先頭アドレスを､第二引数にはユーザ辞書の先頭アドレスを指定してください
        /// インスタンスの開放は自動で行いますが､辞書の開放は手動でしてください
        #[allow(clippy::missing_safety_doc)]
        pub unsafe fn create_ptr<'b>(
            &self,
            sysdic: *const c_void,
            userdic: *const c_void,
        ) -> Result<AqK2Kinstance<'b>, Box<dyn std::error::Error>> {
            let mut errcode: i32 = 0;
            let instance = (self.dll.create_ptr)(sysdic, userdic, &mut errcode as *mut i32);
            if instance.is_null() {
                Err(Box::new(AqK2Kerr(errcode)))
            } else {
                Ok(AqK2Kinstance {
                    instance,
                    // dll: Arc::clone(&*(&self.dll as *const _ as *mut Arc<AqK2KDLL2>)),
                    dll: Arc::clone(force_convert!(&self.dll, Arc<AqK2KDLL2>)),
                })
            }
        }
    }

    /// # createやcreate_ptrが返すAqKanji2Koeのインスタンスのラッパー
    /// AqKanji2Koe_ReleaseはDrop時に実行されるため､自分で実行する必要はありません
    pub struct AqK2Kinstance<'a> {
        instance: *mut c_void,
        dll: Arc<AqK2KDLL2<'a>>,
    }

    impl<'a> AqK2Kinstance<'a> {
        /// 本家のAqKanji2Koe_Convert_utf8に当たります
        /// 第一引数には漢字かな混じりのテキストを､第二引数はバッファーサイズで､基本的にはNoneを入れとけば公式推奨の入力テキストの２倍を確保しますが､心配性の方はSome(バイト単位のバッファーサイズ)を指定してください
        pub fn convert<'b>(
            &mut self,
            kanji: &str,
            buffersize: Option<usize>,
        ) -> Result<AqK2Kstr<'b>, Box<dyn std::error::Error>> {
            unsafe {
                let mut size: usize = match buffersize {
                    Some(s) => s,
                    None => (kanji.len() + 1) * 2,
                };
                if size < 256 {
                    size = 256;
                }
                let kanji2 = CString::new(kanji)?;
                let layout = alloc::Layout::from_size_align_unchecked(
                    mem::size_of::<c_char>() * size,
                    mem::align_of::<c_char>(),
                );
                let buffer = alloc::alloc(layout) as *mut c_char;
                let errcode = (self.dll.convert)(
                    self.instance,
                    kanji2.as_ptr(),
                    buffer,
                    TryFrom::try_from(size)?,
                );
                if errcode == 0 {
                    Ok(AqK2Kstr {
                        content: CStr::from_ptr(buffer).to_str()?.forcemut(),
                        layout,
                    })
                } else {
                    Err(Box::new(AqK2Kerr(errcode)))
                }
            }
        }
    }

    impl<'a> std::ops::Drop for AqK2Kinstance<'a> {
        fn drop(&mut self) {
            unsafe {
                (self.dll.release)(self.instance);
            }
        }
    }

    unsafe impl<'a> Send for AqK2Kinstance<'a> {}

    unsafe impl<'a> Sync for AqK2Kinstance<'a> {}

    /// # convert関数で生成された文字列へのスマートポインタ
    /// このスマートポインタを参照外しすると変換された文字列が出てきます
    /// ヒープの開放はDrop時に実行されるため､自分で実行する必要はありません
    pub struct AqK2Kstr<'a> {
        content: &'a mut str,
        layout: alloc::Layout,
    }

    impl<'a> std::ops::Deref for AqK2Kstr<'a> {
        type Target = &'a mut str;

        fn deref(&self) -> &Self::Target {
            &self.content
        }
    }

    impl<'a> std::ops::DerefMut for AqK2Kstr<'a> {
        fn deref_mut(&mut self) -> &mut Self::Target {
            &mut self.content
        }
    }

    impl<'a> std::ops::Drop for AqK2Kstr<'a> {
        fn drop(&mut self) {
            unsafe {
                alloc::dealloc(&mut self.content.as_bytes_mut()[0] as *mut u8, self.layout);
            }
        }
    }

    struct AqK2Kerr(i32);

    impl AqK2Kerr {
        fn msg(&self) -> &str {
            match self.0 {
                100 => "その他のエラー, エラーコード: 100",
                101 => "関数呼び出し時の引数がNULLになっている, エラーコード: 101",
                104 => "初期化されていない(初期化ルーチンが呼ばれていない), エラーコード: 104",
                105 => "入力テキストが長すぎる, エラーコード: 105",
                106 => "システム辞書データが指定されていない, エラーコード: 106",
                107 => "変換できない文字コードが含まれている, エラーコード: 107",
                200..=299 => "システム辞書(aqdic.bin)が不正, エラーコード: 200番台",
                300..=399 => "ユーザ辞書(aq_user.dic)が不正, エラーコード: 300番台",
                _ => "未定義のエラー",
            }
        }
    }

    impl std::fmt::Display for AqK2Kerr {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}", self.msg())
        }
    }

    impl std::fmt::Debug for AqK2Kerr {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "{}", self.msg())
        }
    }

    impl std::error::Error for AqK2Kerr {
        fn description(&self) -> &str {
            self.msg()
        }
    }
}
