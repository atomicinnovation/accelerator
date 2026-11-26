# Split token probe

#### M001 markup

x crates.**io** y

#### M002 markup

x **crates**.io y

#### M003 markup

x crates**.io** y

#### M004 markup

x c**rates**.io y

#### M005 markup

x crates.i**o** y

#### M006 markup

x crates.*io* y

#### M007 markup

x *crates*.io y

#### M008 markup

x crates._io_ y

#### M009 markup

x crates.~~io~~ y

#### M010 markup

x a.b.**io** y

#### M011 markup

x crates.**io**/path y

#### M012 markup

x crates.**io**:80 y

#### M013 markup

x foo.<b>rs</b> y

#### M014 markup

x <b>foo</b>.rs y

#### M015 markup

x foo.<span>rs</span> y

#### M016 markup

x crates.<!-- c -->io y

#### M017 markup

x foo.`rs` y

#### M018 markup

x `foo`.rs y

#### M019 markup

x jql.rs`:167` y

#### M020 markup

x [foo](x).rs y

#### M021 markup

x foo.[rs](x) y

#### M022 markup

x [foo](https://x.io).rs y

#### M023 markup

x crates.io**z** y

#### M024 markup

x **z**crates.io y

#### M025 markup

x foo.<br>rs y

#### M026 remedy

x crates\.**io** y

#### M027 remedy

x **crates**\.io y

#### M028 remedy

x crates**\.io** y

#### M029 remedy

x crates\.*io* y

#### M030 remedy

x crates\._io_ y

#### M031 remedy

x foo\.<b>rs</b> y

#### M032 remedy

x <b>foo</b>\.rs y

#### M033 remedy

x foo\.`rs` y

#### M034 remedy

x `foo`\.rs y

#### M035 remedy

x [foo](x)\.rs y

#### M036 remedy

x foo\.[rs](x) y

#### N001 nfd

x café.io y

#### N002 nfd

x naïve.io y

#### N003 nfd

x ñ.io y

#### N004 nfd

x é.io y

#### N005 nfd

x a.café.io y

#### N006 nfd

x café.com/path y

#### N007 nfd

x café.io:80 y

#### N008 nfd

x foo.café y

#### N009 nfd

x 한국.kr y

#### N010 nfc

x café.io y

#### N011 nfc

x 한국.kr y

#### N012 nfc

x foo.café y

#### N013 marks

x हिन्दी.com y

#### N014 marks

x भारत.in y

#### N015 marks

x ́e.io y

#### N016 marks

x foo.́io y

#### N017 marks

x café́.io y

#### N018 marks

x e⃝.io y

#### N019 marks

x a‍.io y

#### N020 remedy

x café\.io y
