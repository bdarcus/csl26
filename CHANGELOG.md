# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.81.0] - 2026-09-06

### Bug Fixes

**ci**

- Filter skipped styles in fidelity scope ([`807fa54`](https://github.com/citum/citum-core/commit/807fa54eb313a601362aff521982bef75777c308))


**engine**

- Render djot in substituted titles ([`61cb868`](https://github.com/citum/citum-core/commit/61cb868f82cf12bea7b13c2b9b4c21530dbe2d95))

- Include given name in sort key ([`0dab325`](https://github.com/citum/citum-core/commit/0dab32566785776303fc0f723d0418f74bf2c471))

- Compare all co-authors in sort key ([`7c18935`](https://github.com/citum/citum-core/commit/7c1893545da02cbd46924c50ad6ac5e549c88681))

- Emphasize substituted titles ([`3d3fdcc`](https://github.com/citum/citum-core/commit/3d3fdcc7628251ba9f0e7eccfdfc9f1a0936bdc5))

- By-cite is document-wide again ([`e4ef118`](https://github.com/citum/citum-core/commit/e4ef118934449e21dc45f5c7322756c35d0bfe21))

- Escalate givenname past same initials ([`a7f6759`](https://github.com/citum/citum-core/commit/a7f67590e62f66d98b00671178270bed75f17b77))

- Escalate collapse locator delimiter ([`3e5f766`](https://github.com/citum/citum-core/commit/3e5f7665664bebffd8db43e943356951921df8a2))

- Strip djot markup from sort keys ([`bdcb202`](https://github.com/citum/citum-core/commit/bdcb202c1e6bb0cbd5b84402e080790f1eceaf21))

- Recover Chicago multivolume parity ([`5a24f43`](https://github.com/citum/citum-core/commit/5a24f434a4f668f254ef08a8d8042aecc0fb8715))

- Add missing title-case stop words ([`be1c116`](https://github.com/citum/citum-core/commit/be1c1166bca6aeb134a0d8d908a901603d535fb5))

- Numeric collapse needs 3+ consecutive ([`e5fb122`](https://github.com/citum/citum-core/commit/e5fb122ad7ab07411209010feb90eb23ce32c41b))

- Year-only issued sort key ([`066f74e`](https://github.com/citum/citum-core/commit/066f74e4569eb99521f5f49fecfe194d3d86d8ce))

- Status fallback needs a discriminant ([`d2fc47f`](https://github.com/citum/citum-core/commit/d2fc47fe5e68593f177f4519bafe239d108f9f92))

- Widen citation-number sort warning ([`d4eddff`](https://github.com/citum/citum-core/commit/d4eddffdacbe8cf7c4863c38ebbffd181b6f757d))

- Honor render-when on author group ([`3a14272`](https://github.com/citum/citum-core/commit/3a14272bb39b8cb4f5807f60c92d2e0477c2703b))

- Stop double-delimiting item joins ([`c0c71fe`](https://github.com/citum/citum-core/commit/c0c71fe0b79a9a0f543dab29f1c9e089727d81dc))

- Dedupe terminal marks at quote joins ([`f4f6339`](https://github.com/citum/citum-core/commit/f4f6339c5d0d713b376b25f5797b3051683e1494))


**report**

- Show known divergences distinctly ([`279820e`](https://github.com/citum/citum-core/commit/279820e9f499a303b15378e8d763fd35510e2fbe))


**schema**

- Flip elsevier-harvard title-quote ([`8ae8388`](https://github.com/citum/citum-core/commit/8ae83885544cbc4d7fe04b133f790fe6443d2622))

- Flip T&F CSE title-quote ([`cc6f96f`](https://github.com/citum/citum-core/commit/cc6f96fbbb09e39d6182ff99974b719c74b08119))

- Cse disambiguate-add-names migration ([`bf599f8`](https://github.com/citum/citum-core/commit/bf599f80c79d1bb7accafc53ed2b24d043316a49))

- Advertise style shorthands ([`b0a49ee`](https://github.com/citum/citum-core/commit/b0a49ee2ad6212aac9ee9b25115796209a8c5d9c))

- Address shorthand review findings ([`59758c5`](https://github.com/citum/citum-core/commit/59758c58cb191fbaa7200c2d867a56ab380b8a05))

- Split comma-embedded name suffix ([`e850f01`](https://github.com/citum/citum-core/commit/e850f0190663a78245c4be37b2048f4cd74afa5c))

- Add compiler contributor role ([`8f8b95e`](https://github.com/citum/citum-core/commit/8f8b95eaf10a2057a2a5b0ef800488a9b0fc4488))


**scripts**

- Detect bibliography order divergence ([`8b92cf5`](https://github.com/citum/citum-core/commit/8b92cf515c37da7bc8a4819947addbe4497fa081))

- Make oracle-refresh actually save ([`49cc970`](https://github.com/citum/citum-core/commit/49cc970501e17727d3dcbe6ae4cbde0b456f703f))

- Update parity baseline for CI ([`3e93876`](https://github.com/citum/citum-core/commit/3e9387640fb2b35d07205df912f1b7d31040a136))


**security**

- Bump h2 for RUSTSEC-2026-0258 ([`3260a3a`](https://github.com/citum/citum-core/commit/3260a3a3fe9911c76846b10b51781c7d1524a65f))


**styles**

- Enable initials in chicago disambig ([`1c359bc`](https://github.com/citum/citum-core/commit/1c359bc1e11150a1eab02322f5e1907c57ebf479))

- Suppress no-date term for T&F CSE ([`db58091`](https://github.com/citum/citum-core/commit/db58091936a85b11c08f7a7a6782329ef64b2314))

- Scope T&F CSE names to citation ([`bbd0817`](https://github.com/citum/citum-core/commit/bbd08173b2fb83548635ad2827745358a6e98f4e))

- Improve Chicago embedded parity ([`ccc89a8`](https://github.com/citum/citum-core/commit/ccc89a87e5d8ee1d750480e181f8fe1a9a737cf6))

- Apply chicago title-case wave 1 ([`4be4cad`](https://github.com/citum/citum-core/commit/4be4cad69eb1ea3cc0c452fb552952d2d885bb13))

- Quote document/thesis titles (wave 2) ([`a2b2862`](https://github.com/citum/citum-core/commit/a2b2862868fdb616d7524cd1575940c713155ab8))

- Chicago map/graphic no longer quoted ([`cd372f7`](https://github.com/citum/citum-core/commit/cd372f7637f0937755254ad8a43d75ad98d455ab))

- Chicago classic/hearing quoting ([`8cc47b8`](https://github.com/citum/citum-core/commit/8cc47b8b0a39567410948c71fe910b62cdc79744))

- Localize chicago prose messages ([`7c160d5`](https://github.com/citum/citum-core/commit/7c160d52caa080eda0c66430a34b3f59c5a76956))

- Restore citation-number collapse ([`457b79b`](https://github.com/citum/citum-core/commit/457b79b6b30070288e03bb179df5e975dc6d6afb))

- Clear leaked collapse in ams-label ([`d33a829`](https://github.com/citum/citum-core/commit/d33a829031c67078728f69279a310f26e106c6cd))

- Chicago webpage citation substitute ([`01d892d`](https://github.com/citum/citum-core/commit/01d892d3333f7fa6ba16c00a87ba19620b9b8f0b))

- Restore shortnb date detail ([`9bd8187`](https://github.com/citum/citum-core/commit/9bd818753b75cb97e2c0b63e0b5d649f2d699c3c))

- Fix numeric marker-wrap bugs ([`2aefb8d`](https://github.com/citum/citum-core/commit/2aefb8df2fdcc9973af905566a3276eb8d2d0a4f))

- Fix MLA period, translator, titlecase ([`e42eb9a`](https://github.com/citum/citum-core/commit/e42eb9a6b0fc4c9130892136b1c2f87d3f078172))

- Fix IEEE colon and publisher/date ([`51b1d47`](https://github.com/citum/citum-core/commit/51b1d476c4e7325790e943ef0224653507679400))

- Fix elsevier/springer structure ([`f4d2429`](https://github.com/citum/citum-core/commit/f4d24296f9b0d8b0d54fcc47b91f5efdaefd44e1))

- Fix APA suffix, label-case bugs ([`368804e`](https://github.com/citum/citum-core/commit/368804e59c37ca6ef5bb76bcc83c5175797d3db9))

- Fix MLA container-chain comma grammar ([`4478b5b`](https://github.com/citum/citum-core/commit/4478b5b19f7a545120ad4a890e4024775a518105))

- Fix Elsevier fallback date grammar ([`9bbd58b`](https://github.com/citum/citum-core/commit/9bbd58bea3451d9c66d2a1c4b18a454dd72cf47d))



### Documentation

**engine**

- Activate range-collapse model ([`528c1b8`](https://github.com/citum/citum-core/commit/528c1b81cb8b4f0b2248bfe120f5b41d948bc5b9))


**schema**

- Fix initialize-with doc, schema ([`12ea3af`](https://github.com/citum/citum-core/commit/12ea3af7d2881c5aa671665b9f5ff09a83f41de3))


**spec**

- Analyze substituted-value formatting ([`5d294fb`](https://github.com/citum/citum-core/commit/5d294fb3dad539b39523467d8907d86c871b11ba))

- Define fallback policy options ([`86bac96`](https://github.com/citum/citum-core/commit/86bac96b2513b96189cc9d1428973963316732a6))

- By-cite disambiguates document-wide ([`325feee`](https://github.com/citum/citum-core/commit/325feeeaaab5317458d0ecafdf795ea44cee1c65))

- By-cite escalates per position ([`2a7cc69`](https://github.com/citum/citum-core/commit/2a7cc69a247d0fa7ed78f040bddf9550868a407c))

- Register div-017 collapse delimiter ([`12b7098`](https://github.com/citum/citum-core/commit/12b709856ef7e4249682aa7d711e702eef59858d))

- Make same-author collapse opt-in ([`cd2f592`](https://github.com/citum/citum-core/commit/cd2f592617ed0564dd966dbd42e0dd367cf20a43))

- Activate same-author collapse ([`12ee766`](https://github.com/citum/citum-core/commit/12ee766afe9b2d2705a88963daaae9e3d8e3394c))

- Design year-suffix collapse rendering ([`d985b31`](https://github.com/citum/citum-core/commit/d985b31780b9e305643553883b63611fe47ff3ee))

- Add second-field-align layout spec ([`eb36ff7`](https://github.com/citum/citum-core/commit/eb36ff7e0bba8d800731ff0255a63ceb7a16ca6b))

- Bib substitute title formatting spec ([`66df0ca`](https://github.com/citum/citum-core/commit/66df0caa8f9ed5c628733092e4ec758164e4e9ab))

- Range/collapse model, resolved ([`3ebbff2`](https://github.com/citum/citum-core/commit/3ebbff219d1d5aebb079f62cc2d4a0c92b52236f))

- Status-aware date fallback ([`bbb6854`](https://github.com/citum/citum-core/commit/bbb6854092bd0f2dd06976f5bcaf9c2217d5b25e))

- Add locator label-case and attachment ([`4d580bc`](https://github.com/citum/citum-core/commit/4d580bc64f696defdee973ed8b3279bf2f3404fd))



### Features

**engine**

- Resolve fallback policies centrally ([`4d4804e`](https://github.com/citum/citum-core/commit/4d4804ec8edb2b318eb16138ff9a12842dc210b2))

- By-cite escalates names per position ([`a2efa0f`](https://github.com/citum/citum-core/commit/a2efa0f6dd450ed83c99ee8de3f73e560a65644c))

- Same-author collapse becomes opt-in ([`e548600`](https://github.com/citum/citum-core/commit/e548600e3b77620c2b7c083afa80aa617e409ad3))

- Render year-suffix collapse degrees ([`c5e6fb1`](https://github.com/citum/citum-core/commit/c5e6fb158855f9ff9a651ac89d576c981cd71fdd))

- Render second-field-align layout ([`4d6019b`](https://github.com/citum/citum-core/commit/4d6019b8d709b77e62fd071312fae25b6c0ac88d))

- Derive substitute title from node ([`0c2e707`](https://github.com/citum/citum-core/commit/0c2e707b7890578abf849d164991e33b7b48757c))

- Locator inherits range-format ([`f324927`](https://github.com/citum/citum-core/commit/f324927ce550ce1cd5a4837a22dd7a8bed69b6ec))

- Configurable identifier ranges ([`a3f9503`](https://github.com/citum/citum-core/commit/a3f950386a6ec3d6656abe85f6c29371ac497621))

- Year-suffix key ignores date range ([`5dec47f`](https://github.com/citum/citum-core/commit/5dec47fe206a121aadc7c2b2f5cd2c257bead60a))

- Title sort uses literal text ([`8f94ea0`](https://github.com/citum/citum-core/commit/8f94ea03589fcb70f668299ae38ec7fb3bc491ca))

- Add locator label-case and attach ([`faf18fd`](https://github.com/citum/citum-core/commit/faf18fd9bdbb4b99642ac75fc2c6eae1ccc4799a))


**migrate**

- Extract fallback policies ([`99e7b6c`](https://github.com/citum/citum-core/commit/99e7b6cfec32a8e8368d84bcebbd6ea161d04116))

- Map all csl collapse values ([`a507ff3`](https://github.com/citum/citum-core/commit/a507ff335768d93c8454073d3e80580665acf557))

- Map collapse delimiter attributes ([`a8671f5`](https://github.com/citum/citum-core/commit/a8671f54fe86e50575b0f7ccdb39f08c0cee76a2))


**schema**

- Add fallback policy models ([`52296b9`](https://github.com/citum/citum-core/commit/52296b98ef6aa70a224a0ff6c07049253064bfa1))

- Remove legacy fallback fields ([`312571b`](https://github.com/citum/citum-core/commit/312571b08533a298a64c0ec892bc79454fe02b71))

- Opt-in same-author collapse ([`c956fe9`](https://github.com/citum/citum-core/commit/c956fe90d69fecfd8011d2af806e47abf1b11f35))

- Add collapse delimiter fields ([`c18bba9`](https://github.com/citum/citum-core/commit/c18bba923c7e82cd3f4f25be7a66ec2f335c9bff))

- Rename range options ([`195e224`](https://github.com/citum/citum-core/commit/195e224851817bf9dc958070de3d1faefcb6d9b3))

- Add status template condition ([`a125ae6`](https://github.com/citum/citum-core/commit/a125ae6d913750e70b1cee5f66beb172c9fb7578))


**scripts**

- Pair citations with refs fixtures ([`5322eec`](https://github.com/citum/citum-core/commit/5322eec8a3c035f79312ea8b99e9c7a189c95c65))

- Audit substitute-formatting corpus ([`e44ab55`](https://github.com/citum/citum-core/commit/e44ab553774685892562fdce9af9f281826b5d19))

- Score citation components too ([`47419fc`](https://github.com/citum/citum-core/commit/47419fcb3e6168aedb457efda41fd08c3c1c423c))

- Derive embedded parity baseline ([`1614270`](https://github.com/citum/citum-core/commit/16142702d629e4d097b81878f2db0720888459b2))

- Add fixture-refresh fan-out recipe ([`d37e565`](https://github.com/citum/citum-core/commit/d37e5655e94dcb11d7c9d2c5220dc433e8a9509c))

- Mask div-017 in oracle reporting ([`545b844`](https://github.com/citum/citum-core/commit/545b8448adc3285cbaebabf12b80868eeb1af9ba))

- Rank parity residuals by leverage ([`305c2f2`](https://github.com/citum/citum-core/commit/305c2f2ac4f7344704edf39b65bc7b1ebb7d8fc5))

- Add parity residual --diff mode ([`f1ca878`](https://github.com/citum/citum-core/commit/f1ca8788ce0c69f4dc27ee8f073d195c4a274610))


**styles**

- Declare same-author collapse ([`9f7789a`](https://github.com/citum/citum-core/commit/9f7789aeb02528c3f696bd982590aa458c84da6f))

- Declare springer collapse delimiter ([`afd55b1`](https://github.com/citum/citum-core/commit/afd55b1ae0f0b77dd9360e27337fa95110a4a782))

- Declare second-field-align ([`c96d9ff`](https://github.com/citum/citum-core/commit/c96d9ff433056bae135c9a2181dbf1049c3e6e10))



### Styling


### Testing

**engine**

- Pin non-collapsed citation clusters ([`4899f39`](https://github.com/citum/citum-core/commit/4899f39191d326e7974169f00cb55ee9357263a7))

- Pin Html sentence-start last-char ([`e6ab30f`](https://github.com/citum/citum-core/commit/e6ab30fbbcd44dface22356d9914eb1e749e4f27))


## [0.80.0] - 2026-08-13

### Bug Fixes

**ci**

- Run required checks on stacked-PR bases ([`6f7bb3c`](https://github.com/citum/citum-core/commit/6f7bb3c73d23afb3b234a19a220c710d67b049dd))

- Widen fidelity timeout for cold builds ([`ca9ed7f`](https://github.com/citum/citum-core/commit/ca9ed7f662c56499d260f59b00550e384b439216))

- Cover render-affecting audit changes ([`54384d1`](https://github.com/citum/citum-core/commit/54384d13b20ae7b570db9f78c7d88601845246b0))

- Refresh audits during releases ([`9a98083`](https://github.com/citum/citum-core/commit/9a9808389ed920cf4494098e4a38cbd68fb5e429))


**engine**

- Punctuation-in-quote under markup ([`6e99c72`](https://github.com/citum/citum-core/commit/6e99c724a50d2c84610cef3715546a9f369391a6))

- Keep comma after abbreviation period ([`05bfcf8`](https://github.com/citum/citum-core/commit/05bfcf8971edb7dc1feb02496cfcc8536f26bb9e))

- Year-suffix follows render order ([`b3dec80`](https://github.com/citum/citum-core/commit/b3dec8049c7163279c49a98bff7588589ab90120))

- Add missing Narrator ContributorRole ([`f5e75a2`](https://github.com/citum/citum-core/commit/f5e75a2f7fbfaf20aa53e2987ba5f5e90e3fe4ef))

- Move citation suffix into quotes ([`6bdd0a5`](https://github.com/citum/citum-core/commit/6bdd0a511ba86b4e07fdb129d652653308ded80d))

- Unify sentence case markup path ([`984187d`](https://github.com/citum/citum-core/commit/984187d2df8b37f962388687886a04dbb81c9d99))

- Grouping follows resolved date slot ([`8d6a051`](https://github.com/citum/citum-core/commit/8d6a0511d442e4f77cbab1f40259b1ad215960cf))


**release**

- Use install without lockfile ([`eda453f`](https://github.com/citum/citum-core/commit/eda453f093d35379f08ffe16581539351f3bdb77))

- Initialize audit submodules ([`89b75df`](https://github.com/citum/citum-core/commit/89b75dfc3f60797243ce422b4a488394ef933d1a))


**report**

- Validate managed renderer binaries ([`cb393a6`](https://github.com/citum/citum-core/commit/cb393a6d59280d4fce9c4e3e353d11825aef1c12))


**schema**

- Type title category mappings ([`b996fb6`](https://github.com/citum/citum-core/commit/b996fb69534ee316a4039ed8ab8a7935f9789137))

- Remove all type-selector keyword ([`b460bc6`](https://github.com/citum/citum-core/commit/b460bc6f98d14820830d7b508ca8398623ee4ef5))


**security**

- Patch lru use-after-free ([`b322aa7`](https://github.com/citum/citum-core/commit/b322aa79f065045b489dc3588931cc6377a36a8a))


**styles**

- Localize chicago contributor roles ([`865c269`](https://github.com/citum/citum-core/commit/865c2693cce78b8073cbb42781052767091fb099))

- Quote chicago newspaper/thesis titles ([`a98c891`](https://github.com/citum/citum-core/commit/a98c89156cfec393643a86c5c022f5d6c0346abc))

- Chicago shortened bib separator ([`4efa2d0`](https://github.com/citum/citum-core/commit/4efa2d0ffb4be39193da15bf92e48ded1d225281))

- Tune chicago journal issue grammar ([`ae3cd2f`](https://github.com/citum/citum-core/commit/ae3cd2f644570a19d12db056d69aea9cf2b52b2a))

- Improve shortened notes parity ([`4689143`](https://github.com/citum/citum-core/commit/4689143c66f9a50540e2ac918fd20f091571c4b4))

- Add numeric label-wrap/separator ([`3b0914c`](https://github.com/citum/citum-core/commit/3b0914c6b321efb238971521253f3eb0565d4238))

- Springer-vancouver title case as-is ([`732aa8d`](https://github.com/citum/citum-core/commit/732aa8d3f00d31190b7e12fc10ff0f1ee906a3a8))



### Documentation

**report**

- Audit shortened-notes coverage ([`5287d8c`](https://github.com/citum/citum-core/commit/5287d8c8554fa3426f4c3975ed9f91fd8c7166d5))


**spec**

- Specify EDTF-keyed locale date names ([`1e085f4`](https://github.com/citum/citum-core/commit/1e085f435f51b2088a01c8ce481c6bb5f86550a6))

- Record punctuation-in-quote markup fix ([`22894b4`](https://github.com/citum/citum-core/commit/22894b4d80c2ae2800594c289ec05cfc9f35ebd9))

- Specify processor-owned markers ([`678bb68`](https://github.com/citum/citum-core/commit/678bb68e17959a2df4b989ce25be6600a067d624))

- Define typed title mappings ([`005f074`](https://github.com/citum/citum-core/commit/005f0741bfe7eddd1f985c06312de9297382aec6))

- Remove all type-selector keyword ([`81555bb`](https://github.com/citum/citum-core/commit/81555bba89a1a3071aaf213450d1668f265c6074))

- Year-suffix order under resolved sorts ([`bc72b51`](https://github.com/citum/citum-core/commit/bc72b51eadee4439834d46f14d46e63a3be5d635))

- Chicago strategy, localization policy ([`c472d51`](https://github.com/citum/citum-core/commit/c472d51fc29ef956ed466ba7386593ea5a243c46))

- Chicago variant axes mapping ([`36c2182`](https://github.com/citum/citum-core/commit/36c21827f0933c5c52583765c2496c1a53fa30d7))

- Define template parity coverage ([`c84ca6b`](https://github.com/citum/citum-core/commit/c84ca6b5899e29e7e82145a39afea9988c2dfe70))

- Define date substitution ([`7507ba7`](https://github.com/citum/citum-core/commit/7507ba76f896eb5c5484f21a26558fdcadc8a842))

- Specify two-name policy ([`0bb3699`](https://github.com/citum/citum-core/commit/0bb36990bfb7f051d85bf41dd788fd9958976bcf))



### Features

**engine**

- Add day-zero-pad date option ([`5dbe34d`](https://github.com/citum/citum-core/commit/5dbe34d752d4488b5fa6f98015828bdde04336f4))

- Implement date substitution ([`05c4322`](https://github.com/citum/citum-core/commit/05c4322a82cf2f6974bd30b02c9ff2cb76114d0d))

- Warn on bare bibliography markers ([`13508e8`](https://github.com/citum/citum-core/commit/13508e8196978eb85d4d53debff27e81bb7b525b))

- Locale-derived punctuation-in-quote ([`464e732`](https://github.com/citum/citum-core/commit/464e73222cdeb8367113d7e487f79ff14d068d2e))


**locale**

- Mark fallback-substituted locales ([`46755d3`](https://github.com/citum/citum-core/commit/46755d37b3fc8d810f90c08c3b7350a508599beb))


**report**

- Add auditable coverage packets ([`a74481a`](https://github.com/citum/citum-core/commit/a74481a5cd353e7e26a79be502f21c50dd2be738))

- Integrate coverage audits ([`306d422`](https://github.com/citum/citum-core/commit/306d4229d544c924bcdf0f3246c3829a0cb37645))

- Add post-change audit evidence ([`a869fd9`](https://github.com/citum/citum-core/commit/a869fd9526d4a15c77c2b9f4c8248cc72339697e))


**schema**

- Key locale date names by EDTF code ([`c583483`](https://github.com/citum/citum-core/commit/c5834838838f263a0889eacc1219fa250e6ebf41))

- Own markers in the processor ([`6595cf0`](https://github.com/citum/citum-core/commit/6595cf0b1b12ebf0e5f2409590354a620b4e0705))

- Support fallback template diffs ([`54d735f`](https://github.com/citum/citum-core/commit/54d735f29b3edfcb952d751569b98d882582f852))

- Add two-name policy ([`7ea608b`](https://github.com/citum/citum-core/commit/7ea608b99203e18db47ae44fb3578dfd24f5d5a6))


**scripts**

- Add STYLE010 localization lint rule ([`8f00c33`](https://github.com/citum/citum-core/commit/8f00c330d39643633966a358e2e86dc41169d4b4))



### Refactor

**styles**

- Ieee titles via category config ([`613ffb9`](https://github.com/citum/citum-core/commit/613ffb94f73388b294aaf22913b2488d68e3d1fa))

- Use date substitute presets ([`4afe520`](https://github.com/citum/citum-core/commit/4afe520651f08284ceb8022cc68eb945d1e7323e))



### Testing

## [0.79.0] - 2026-08-02

### Bug Fixes

**ci**

- Cover exact parity enforcement ([`928e99e`](https://github.com/citum/citum-core/commit/928e99edd7d8814a972ac3cca18b6c45a99c2385))

- Regen schema and oracle baseline ([`91f17f4`](https://github.com/citum/citum-core/commit/91f17f41240c6deb9db4672f580c1c2cca750902))


**engine**

- Align title-case stop words ([`3933dd9`](https://github.com/citum/citum-core/commit/3933dd9536d1de82d9924a4e2148b5566706dc5b))

- Fix punctuation-in-quote joins ([`933761d`](https://github.com/citum/citum-core/commit/933761ddd9b17244ec85ea9b1ee682cab05aa283))

- Bare numeric labels leak a separator ([`ae565ba`](https://github.com/citum/citum-core/commit/ae565baa5978dc9b88ac04669799ba079e12a16b))

- Honor bibliography-free styles ([`9208a6c`](https://github.com/citum/citum-core/commit/9208a6caa7730bb2382f3e61492ffe96284b3af3))


**hooks**

- Drop redundant --base flag from alint ([`4e8bf9c`](https://github.com/citum/citum-core/commit/4e8bf9c5ed7f11a429a47dea287e1650adebf33a))


**migrate**

- Convert citeproc html outside titles ([`e06b7e2`](https://github.com/citum/citum-core/commit/e06b7e2d1056a798735cb64e88b06ce8915be458))


**report**

- Make oracle locale explicit ([`65cb95b`](https://github.com/citum/citum-core/commit/65cb95b71aa5e12d84b6190af389e7dbec143f86))

- Restore canonical parity data ([`51e1be8`](https://github.com/citum/citum-core/commit/51e1be8147f603b2393efd04f3e1796d2589839e))


**schema**

- Add direct-build contributor fold ([`4c67fc0`](https://github.com/citum/citum-core/commit/4c67fc0848e1418ef3c1c2b056d605b17fef273b))

- Merge multilingual config field-wise ([`96d5d45`](https://github.com/citum/citum-core/commit/96d5d455a1f5522b4de093a2a989efccbd914be3))

- Preserve raw_yaml on every load path ([`c09f5bd`](https://github.com/citum/citum-core/commit/c09f5bdac01275c7422e4e576a9ec88b1c9a88a5))

- Correct conference page punctuation ([`364cb90`](https://github.com/citum/citum-core/commit/364cb908c295632278fd7d7280056434ebe5dce8))


**styles**

- Correct multilingual example errors ([`886ff18`](https://github.com/citum/citum-core/commit/886ff1810b50b0fb6d883d6946edf3084fec6151))

- Dedupe gb-t-7714-2025 dates block ([`8862075`](https://github.com/citum/citum-core/commit/8862075657761d53c2a7d370a31e68e2bfd591e3))

- Scope T&F sentence-case to component ([`a9c7e06`](https://github.com/citum/citum-core/commit/a9c7e062c0d16582db7acd6ce74b0acfb5fed836))

- Clear inherited monograph emphasis ([`418f35a`](https://github.com/citum/citum-core/commit/418f35afde24f810b82b62d12af70927ee7e7142))

- Fix two title deep-merge regressions ([`0b8efb7`](https://github.com/citum/citum-core/commit/0b8efb775360576a694c8ae810574c11d80f71cc))

- Fix split-corpus fallout ([`b636fd6`](https://github.com/citum/citum-core/commit/b636fd6c5b687e714bf396152c5bcc4d06253fa2))

- Elsevier-vancouver label-mode leak ([`14fe4c2`](https://github.com/citum/citum-core/commit/14fe4c2c0c42847f0708910ca78bb7d998c5eda8))

- Add bibliography numbering to AMA ([`fcdff9c`](https://github.com/citum/citum-core/commit/fcdff9cbd597dfac2ee641cee8c346d982d8e727))

- Bib numbering, nature/ACS/AMA-alpha ([`c97833a`](https://github.com/citum/citum-core/commit/c97833a93f03f1f441d0a0cc67f6601645a966ad))

- Use label-mode for numeric labels ([`29e1b72`](https://github.com/citum/citum-core/commit/29e1b72d472fde5fb164b2ceb838984e2900b3bb))

- Bracket Chicago guessed-date years ([`5b47090`](https://github.com/citum/citum-core/commit/5b4709068efd9310c383239afcc45520a8dc89fb))

- Improve Chicago exact parity ([`8a81ca5`](https://github.com/citum/citum-core/commit/8a81ca58a6bfc21d9d176743bdbea5d00c47c347))

- Retain Chicago link suffixes ([`92fcfaf`](https://github.com/citum/citum-core/commit/92fcfafe87cba27f67516b92af675f88427f8f88))

- Terminate Chicago bibliography links ([`940b461`](https://github.com/citum/citum-core/commit/940b461df980a9e430d468198418391ff97b4660))



### Documentation

**report**

- Specify inheritance view ([`2668a98`](https://github.com/citum/citum-core/commit/2668a98e72a10609babbe03edba8e5bc58f4e66a))

- Refresh compat.html read-this copy ([`4a094ef`](https://github.com/citum/citum-core/commit/4a094efebd55c04a4a6cfdfaffbccb31df86aa3f))

- Spike class-A parity gap ([`b97de7e`](https://github.com/citum/citum-core/commit/b97de7ea35430bf5debeeb18159fb0db8bd0d043))


**spec**

- Add unified style inheritance spec ([`0be290f`](https://github.com/citum/citum-core/commit/0be290f64d9edfd2c10a13b728991b95e895444f))

- Record deep-merge constraints ([`b358d8c`](https://github.com/citum/citum-core/commit/b358d8c566364f46f3aceee77e8d844a8f683ba5))

- Require format-neutral raw ingest ([`5cf3740`](https://github.com/citum/citum-core/commit/5cf37400183e5d4534248a3f5cf4d3617e8d5dbc))

- Flip style inheritance spec to active ([`894d8e6`](https://github.com/citum/citum-core/commit/894d8e622b8d301fed9cbae915599d6c0709b007))


**styles**

- Audit inheritance and portfolio ([`296d11c`](https://github.com/citum/citum-core/commit/296d11c00a38c7a72059e3e697067f94c21b0a80))

- Explain the three-tier portfolio ([`7f8d014`](https://github.com/citum/citum-core/commit/7f8d014b3a8e91b10a610bede02659dbe6fb812a))

- Align exact parity workflow ([`bb47b5e`](https://github.com/citum/citum-core/commit/bb47b5e47108e9d2070c41e508d1001550696e5b))



### Features

**engine**

- Configure EDTF year ranges ([`9c6114d`](https://github.com/citum/citum-core/commit/9c6114d4c6be0d6926253aef79e3a69af9827ecc))

- Type separators at punctuation joins ([`d9e59d8`](https://github.com/citum/citum-core/commit/d9e59d891bb5af984dbb167ab49411eb59675b8f))


**locale**

- Label biblatex editorial roles ([`d77655a`](https://github.com/citum/citum-core/commit/d77655a2d78062d8d6a4adefb85b50d1ad8d80c1))


**refs**

- Preserve biblatex field semantics ([`c90a5f2`](https://github.com/citum/citum-core/commit/c90a5f202e460ed3ecbce4ac14dbb156d593f190))

- Route biblatex to native classes ([`6c7282a`](https://github.com/citum/citum-core/commit/6c7282ae3ff4e8f18ce9f6f172a853806afad9c0))


**report**

- Surface style inheritance ([`bbf5a0b`](https://github.com/citum/citum-core/commit/bbf5a0bdf04ca65112d19250b99e3ad49f214d45))

- Focus embedded compatibility ([`82e3d76`](https://github.com/citum/citum-core/commit/82e3d76fd90c5b0b61024da84b0e2b5a87bf6d32))

- Measure elsevier/T&F core overlap ([`828cb9d`](https://github.com/citum/citum-core/commit/828cb9d2046312e3338d48e0fcad008ca0a1b436))

- Harden exact parity gate ([`4129b71`](https://github.com/citum/citum-core/commit/4129b71ebb65a7f68278ff9eac95885ba779bb8b))


**schema**

- Deep-merge options on extends ([`303a38f`](https://github.com/citum/citum-core/commit/303a38f02863bab5894cb6130cab87178908b4ac))

- Flush numeric bibliography labels ([`79a66fa`](https://github.com/citum/citum-core/commit/79a66fafb0fbc7a3e8e4d8d4d0503d308cd60fa0))

- Add locator and title presets ([`e51d11b`](https://github.com/citum/citum-core/commit/e51d11bf564fbcc71bc531f0cb6f4b55791b6ed2))


**styles**

- Split community style corpus ([`0eec168`](https://github.com/citum/citum-core/commit/0eec16875b4f446f4a337f592d7bce064c99e4ab))

- Field-merge runtime scope cascade ([`85e94e8`](https://github.com/citum/citum-core/commit/85e94e81eec6912cff77fed7fcb8271cb058b758))


**tooling**

- Rework config-presets matching ([`5c25db3`](https://github.com/citum/citum-core/commit/5c25db3a97b471e8fd17a321d9ee848cf6d85f2d))



### Refactor

**refs**

- Unify biblatex under formats/ ([`48a0982`](https://github.com/citum/citum-core/commit/48a09820279634f2491ea61bd3cb314c9901c389))

- Declarative mapping tables ([`9b28345`](https://github.com/citum/citum-core/commit/9b283455f95c554498d98a68bf0af44150b89280))



### Testing

**engine**

- Fix domain_fixtures raw-yaml bypass ([`77b1c14`](https://github.com/citum/citum-core/commit/77b1c145e96e5d284ef8bbb7cb8ce5f9d50a3222))


**refs**

- Add biblatex conversion corpus ([`6634f9f`](https://github.com/citum/citum-core/commit/6634f9f3cf6dc51982ca755c6e50cb86ffde435e))


## [0.78.0] - 2026-07-24

### Bug Fixes

**ci**

- Enforce squash merge commit policy ([`bd19b78`](https://github.com/citum/citum-core/commit/bd19b7884f14d67969891c18d081327a33785cd7))

- Pin checkout action v6 ([`059f9ee`](https://github.com/citum/citum-core/commit/059f9eea92ec58dfcea76a27aa7105144471ae88))


**engine**

- Restore locale type-variant fallback in bilingual styles ([`41ce890`](https://github.com/citum/citum-core/commit/41ce890a067ea782fa636592df73e26add98f19c))

- Tailor case mapping by locale ([`93ff216`](https://github.com/citum/citum-core/commit/93ff21681437f233a2854a0ddf0bf78ca838dffd))

- Exempt dates from component dedup ([`e6d7adb`](https://github.com/citum/citum-core/commit/e6d7adba254b6e56f339ceb0b4dc3286e037d06c))

- Scope disambig grouping to bib config ([`a306577`](https://github.com/citum/citum-core/commit/a306577c9115b2f085004152ee9059e0f88a45e9))

- Apply year-suffix to date fallback ([`7039c37`](https://github.com/citum/citum-core/commit/7039c37f4ae2eccbb1bf6f4b45b6c7d0479e445e))


**hooks**

- Honor primary history for mirrors ([`80c1c72`](https://github.com/citum/citum-core/commit/80c1c72bef11afc87eb6ce532d13bb5710cd2e26))


**migrate**

- Convert citeproc html titles to djot ([`14c1651`](https://github.com/citum/citum-core/commit/14c1651b4bbb357f32cbfd989fcb86835238bd6e))


**report**

- Add GB/T date-annotation divergence ([`0e9c8c4`](https://github.com/citum/citum-core/commit/0e9c8c42cbad57fd70b66cce36c6d5e5b6ba546e))

- Stop masking punctuation-only defects ([`7c5ca94`](https://github.com/citum/citum-core/commit/7c5ca948a8b7e9ab904049ee49cbf9548d2efc56))


**styles**

- Rebuild GB/T author-date variants ([`9ac83a1`](https://github.com/citum/citum-core/commit/9ac83a1ceb4f867b3a56995a7b43f1ffa7a841d5))

- Move GB/T anonymous term to locale ([`165142c`](https://github.com/citum/citum-core/commit/165142c64b169eaa3cb270a2fe0a58b5ca503eb7))

- Gb/t 7714 entry terminal period ([`71c06a3`](https://github.com/citum/citum-core/commit/71c06a3ceded00977121cd1772db8515d6b662ed))

- Fill gb-t-7714 numeric bib gaps ([`b6849c6`](https://github.com/citum/citum-core/commit/b6849c636c6b69030a02c4e0e899703850338822))

- Add missing gb-t-7714 type-variants ([`edcdcf4`](https://github.com/citum/citum-core/commit/edcdcf46184f8a36a09ef5641b01185898502b8d))



### Documentation

**spec**

- Activate PER_ITEM_TERM_LOCALE ([`8e2b841`](https://github.com/citum/citum-core/commit/8e2b8414db2a581437a1ec34f505603356cd4248))

- Add bean on note-wrap config redesign ([`0cd8762`](https://github.com/citum/citum-core/commit/0cd87622ae49cd93a5c560441065f5801a2e9c83))

- Fix broken calendar-note spec link ([`cc4c3b7`](https://github.com/citum/citum-core/commit/cc4c3b7943aca366cc0c8a58cbe55a076c563a97))



### Features

**engine**

- Support term-locale: item ([`9acb619`](https://github.com/citum/citum-core/commit/9acb61968ab03cbd22caad9d41487e510c03b288))

- Warn on unavailable item term locale ([`e6326d6`](https://github.com/citum/citum-core/commit/e6326d626d8d1da3bb60b3879be930ef8e9d144c))

- Add punctuation width presets ([`2f2e4b2`](https://github.com/citum/citum-core/commit/2f2e4b27504daf688eee7fb886c208b9d1a9268f))

- Suppress date note per component ([`93e5d60`](https://github.com/citum/citum-core/commit/93e5d60e0d72d1ee7c55690baffcdc21649399ec))

- Prefer locale message for anonymous ([`a064398`](https://github.com/citum/citum-core/commit/a064398b79d2d976c09439307be9fb94f68be573))


**locale**

- Add localized digit systems ([`7a16188`](https://github.com/citum/citum-core/commit/7a1618809f16d082abed6b82c53d97ed71f7b874))

- Add punctuation realization spacing ([`9240973`](https://github.com/citum/citum-core/commit/9240973ee314e38005575a26f53de5c1c43c0aa5))


**schema**

- Add multilingual term-locale opt-in ([`082390a`](https://github.com/citum/citum-core/commit/082390a7d83d302a83692b02c131cee5d5519900))



### Testing

**engine**

- Use zh-CN locale in era fixtures ([`505c460`](https://github.com/citum/citum-core/commit/505c460c5c8ea365f3c95e3efe026d059f5bd8d7))


## [0.77.0] - 2026-07-21

### Bug Fixes

**engine**

- Separate component editor fallback ([`aab5772`](https://github.com/citum/citum-core/commit/aab57727bacf24cbfbc7ae9cee214d22c585f900))


**locale**

- Localize ordinal number forms ([`c5b21d4`](https://github.com/citum/citum-core/commit/c5b21d4fec3e7adc4e1e2c9c7582f7e65c4121b9))

- Fill zh-CN/ar-AR locale gaps ([`1e39658`](https://github.com/citum/citum-core/commit/1e3965858f9242528c8881a4312c786a11bf1f38))


**scripts**

- Gate on adjusted fidelity counts ([`1d3e7df`](https://github.com/citum/citum-core/commit/1d3e7df199ea2c022c6d117dd95285af55b7859c))


**styles**

- Gb/t latin refs get latin punctuation ([`f60d8aa`](https://github.com/citum/citum-core/commit/f60d8aa299280f2d7bdad279a38c986d1b2f1117))



### Documentation

**migrate**

- Add migration approach review audit ([`4f20726`](https://github.com/citum/citum-core/commit/4f20726da64d2aa5c511fde5d1b63b2a2eb2a1fd))

- Record tier-0 alias negative result ([`558c67e`](https://github.com/citum/citum-core/commit/558c67e20aff58314ac8f8fb1355b4de5a46b21a))


**spec**

- Define GB/T date model and policy ([`0b8d69a`](https://github.com/citum/citum-core/commit/0b8d69ac4a9039492e989d411c0f3f8d2bd9f093))

- Define punctuation realization layer ([`a3deac2`](https://github.com/citum/citum-core/commit/a3deac25c5f8f9d85e92578a364b80a4c87d73e5))

- Define per-item term localization ([`52dcd0d`](https://github.com/citum/citum-core/commit/52dcd0da61096bcf6aff1bec97e7c0011cf9e956))

- Define bidi output handling ([`84a6cc3`](https://github.com/citum/citum-core/commit/84a6cc319173a2580feb2b7d49f824d3556de3e5))

- Add date annotation as a note ([`dd8afb0`](https://github.com/citum/citum-core/commit/dd8afb07b43ecdacc5a346b8d709b22fff077f28))

- Document bare-note extension point ([`1d4f1d1`](https://github.com/citum/citum-core/commit/1d4f1d15139aef6037f14e590fb78f5f6daf4f48))

- Amend punctuation-width bean ([`ca2ffcf`](https://github.com/citum/citum-core/commit/ca2ffcffabfb74a0815e28f17d52057b2d16adc0))



### Features

**engine**

- Close GB/T numeric fidelity gaps ([`9004614`](https://github.com/citum/citum-core/commit/9004614c1c1bc45c9dc49679d2685ebf4d031e32))

- Add script-aware punctuation remap ([`c6bd91c`](https://github.com/citum/citum-core/commit/c6bd91cbbc2cd50c7ff44845d261327d03a45a82))

- Resolve ISO 15924 scripts ([`d689d19`](https://github.com/citum/citum-core/commit/d689d19e47e154bd5858d168b66f7df42a1b38b6))

- Realize script-aware wrap glyphs ([`721f571`](https://github.com/citum/citum-core/commit/721f57107c70232c3c0b9d4d04cc65cfaeae343e))

- Realize semantic punctuation ([`187e8eb`](https://github.com/citum/citum-core/commit/187e8ebc7176af2e2620261fd8c02dd78473704f))

- Render opaque date notes ([`4478585`](https://github.com/citum/citum-core/commit/4478585be91bc64e0d644b166beae9cb1e482d53))


**locale**

- Add ja-JP/ko-KR/ru-RU locales ([`3d795a0`](https://github.com/citum/citum-core/commit/3d795a0ffeeeb7bac016b560045576f30d81fe52))


**migrate**

- Support CSL-M layouts ([`63fdb10`](https://github.com/citum/citum-core/commit/63fdb1047c8090f4cd97dccc3f2284183275b7a2))


**schema**

- Add CSL-M foundations ([`b44c6ac`](https://github.com/citum/citum-core/commit/b44c6ac0518dd15207e6f7e9cd776aeb9e74818c))

- Preserve GB/T reference data ([`3bff0dd`](https://github.com/citum/citum-core/commit/3bff0ddf72c5c70fe59d35d4c3f7ba87e82c3169))

- Preserve GB/T monograph metadata ([`0057f0f`](https://github.com/citum/citum-core/commit/0057f0f1546d2c208fc8daac1358f605d82f7638))

- Support GB/T date ranges ([`fe8cfe0`](https://github.com/citum/citum-core/commit/fe8cfe093d91bf25b666f25e73c5cd621ce6e5d5))

- Support GB/T date roles ([`22e044b`](https://github.com/citum/citum-core/commit/22e044bbaba50187c83b921ddee26c6f096d9b5e))


**styles**

- Add GB/T 7714 family ([`364be5f`](https://github.com/citum/citum-core/commit/364be5f81d43845f9bc05b40c2176158555b230f))

- Extend GB/T note-wrap rollout ([`e23e098`](https://github.com/citum/citum-core/commit/e23e0989e1d73d2b65e7a20eaa2645332a7488a6))


**tooling**

- Wasm bench build + jsr/session opts ([`81b2a03`](https://github.com/citum/citum-core/commit/81b2a03d778a290d34c98860373482f8af1e5463))



### Performance

**engine**

- Cache locale expander per thread ([`36c0b65`](https://github.com/citum/citum-core/commit/36c0b65999824a8e7ccb475a25727ee95fc2229f))



### Refactor

**styles**

- Use semantic punctuation ([`13aa4d6`](https://github.com/citum/citum-core/commit/13aa4d6acaa84430fb4fc13261e04e2d94e9a16e))


## [0.76.0] - 2026-07-15

### Bug Fixes

**cli**

- Add installer component flag ([`2085ea3`](https://github.com/citum/citum-core/commit/2085ea366a12f2dde737a9f1b6bc1f653ff6b589))


**release**

- Recover crate publishing ([`47600a9`](https://github.com/citum/citum-core/commit/47600a967ee4ecdffea0862f6c2c7aa3a63a8d84))

- Use bash for tag recovery ([`ec3f462`](https://github.com/citum/citum-core/commit/ec3f462c08d3c3f8dfcf6a42c1ae77743711b10e))



### Documentation

**spec**

- Define primary substitution ([`a9abf97`](https://github.com/citum/citum-core/commit/a9abf976d8251a5b01d7bba6d0abc89ca1cdc2c8))



### Features

**schema**

- Support cross-role contributor lists ([`a66af4a`](https://github.com/citum/citum-core/commit/a66af4a1fe093f1878779103a2a2d30b760ae58c))



### Build

**schema**

- Add unicode normalization ([`c0bbbee`](https://github.com/citum/citum-core/commit/c0bbbeeae68454f38bc55bfaa0f640a52fd2b541))


## [0.75.0] - 2026-07-14

### Bug Fixes

**ci**

- Scope spin yank exception ([`f91f422`](https://github.com/citum/citum-core/commit/f91f42237a98df9724aa0d908e19f98a6290e212))

- Use issue-id input for vouch manage ([`a302e2a`](https://github.com/citum/citum-core/commit/a302e2a66420aa29822029be4675524efd7532d4))


**cli**

- Ship citum-migrate via gnu on Linux ([`13464eb`](https://github.com/citum/citum-core/commit/13464eb9a746125155edb6bdefa59e2b8f5af35b))


**engine**

- Span-aware punctuation cleanup ([`07f4279`](https://github.com/citum/citum-core/commit/07f4279919e3928da470820d1df8cbda76c3e24d))

- Markup-aware citation delimiter ([`dad1d9d`](https://github.com/citum/citum-core/commit/dad1d9d3b3d7eac2c6e820127b9dbd15309d9f9f))

- Year-suffix order follows sort policy ([`14a52da`](https://github.com/citum/citum-core/commit/14a52da286a4e41293e2cba14ebaba6f3d59a1f6))



### Documentation

**spec**

- Add cross-role contributor lists spec ([`9504c9f`](https://github.com/citum/citum-core/commit/9504c9f702985140a3602ea3d13dcfab2309b27c))

- Add render-when contract ([`b9cb1be`](https://github.com/citum/citum-core/commit/b9cb1be335d637d0b56ace035c93089a8f6eae94))



### Features

**engine**

- Add format-aware visible-text hook ([`beb5b48`](https://github.com/citum/citum-core/commit/beb5b48e48c482fb5e914660ff874bb3cac8a39f))


**locale**

- Add math locator terms ([`bb85e91`](https://github.com/citum/citum-core/commit/bb85e9160a3913fe8183c6328a5ad00b75443788))


**schema**

- Localize punctuation collisions ([`1d98fdf`](https://github.com/citum/citum-core/commit/1d98fdf099c3248b27da183275fa25b0dd625e2a))

- Add bib info version field ([`5226770`](https://github.com/citum/citum-core/commit/5226770589967cb2c0f24575e2fd9df87ae82710))

- Add strip-protocol link option ([`09c51ba`](https://github.com/citum/citum-core/commit/09c51baec04a70ad5203fd0aa5d98f82058b9997))

- Add family-first-except-last order ([`22ab802`](https://github.com/citum/citum-core/commit/22ab80212956d269aa9df77ffebd2f4ec60df63b))

- Validate render-when conditions ([`de454e0`](https://github.com/citum/citum-core/commit/de454e0512b6d8e2564b552c79dd726043f2b3a1))


## [0.74.0] - 2026-07-11

### Bug Fixes

**engine**

- Mangle ffi symbols in test builds ([`b5df83d`](https://github.com/citum/citum-core/commit/b5df83d2b309b2b1fd0a24a3470a7558930c81c2))

- Preserve unresolved sorted citations ([`d0f2cb5`](https://github.com/citum/citum-core/commit/d0f2cb53d1fda4b73e65796634184b93348075ac))

- Unify bibliography group routing ([`a44b04d`](https://github.com/citum/citum-core/commit/a44b04d3b34abf9b3fe236d5bb3e5ad217a20f41))

- Keep compound row for cited members ([`8b661a4`](https://github.com/citum/citum-core/commit/8b661a46d9eafef1d21c77b36bdec85505f06087))


**scripts**

- Resolve citum-server from PATH ([`3e37008`](https://github.com/citum/citum-core/commit/3e370086a5f78192eec2ca5ebecac4084782b876))



### Documentation

**spec**

- Add multilingual sorting spec ([`b789c9f`](https://github.com/citum/citum-core/commit/b789c9f437b89df8d49135d8577864d33dad7f9b))



### Features

**engine**

- Support multilingual sort keys ([`9d039b3`](https://github.com/citum/citum-core/commit/9d039b337fe4892e9979421e2b77bb7aef119317))

- Parallel bibliography rendering ([`fb190cf`](https://github.com/citum/citum-core/commit/fb190cf17dda8671bbc0f4c9a4c1e4023c2332f2))



### Performance

**engine**

- Hoist configs, make parallel opt-in ([`9283534`](https://github.com/citum/citum-core/commit/9283534cbb090c7a5fa7a12699ef27b60c3f560b))

- Render bibliography entries once ([`17eb6f6`](https://github.com/citum/citum-core/commit/17eb6f6e5ef48686ff8d66df45206660fd017e84))

- Cache sorted-ID spine per doc call ([`2c0dea1`](https://github.com/citum/citum-core/commit/2c0dea15999b1c405768bb2a9365de77844a7e9b))



### Refactor

**engine**

- Move render fields into RunState ([`e99d5bd`](https://github.com/citum/citum-core/commit/e99d5bd3d79d19a00d0f91e27b2a8642055c71c0))

- Make render-run state explicit ([`2399218`](https://github.com/citum/citum-core/commit/23992188468f5558b3488fa2cb41b8aff5f8dea8))

- Use Arc for config sharing ([`84211bc`](https://github.com/citum/citum-core/commit/84211bc5f8c8629d69c824e6261ede1451d3a12c))

- Make run-state maps thread-safe ([`5824ac2`](https://github.com/citum/citum-core/commit/5824ac2590908abe9f8f9f378553ac562b14dc46))


## [0.73.0] - 2026-07-07

### Bug Fixes

**ci**

- Replace wretry.action with manual retry ([`58c846f`](https://github.com/citum/citum-core/commit/58c846feface11ad98b0479d7f0747489a689581))


**engine**

- Key disambiguation cache by id ([`f42eec9`](https://github.com/citum/citum-core/commit/f42eec98d41eabe8859335fc239cf92e4c45354f))

- Suffix undated year labels ([`7d949db`](https://github.com/citum/citum-core/commit/7d949db62e38d29be8500d2abde5de075665006e))

- Dedupe note pipeline; use locale rule ([`cc39e31`](https://github.com/citum/citum-core/commit/cc39e313d7175ffcdb4dc551a0fd7a55eabc2707))

- Honor delimiter-precedes-last ([`4a5c5d8`](https://github.com/citum/citum-core/commit/4a5c5d8eccf057ade3c725672c4fac34484687a6))

- Suffix joins with sort-separator ([`4e9dea3`](https://github.com/citum/citum-core/commit/4e9dea38fcc2820937b7743af9c6ae98031d8fd4))

- Dedupe leading grouping contributor ([`24ba58d`](https://github.com/citum/citum-core/commit/24ba58d0b2dd8f4455e02506a6168149ad6a0f7a))


**migrate**

- Consume shared title-category table ([`732f3e8`](https://github.com/citum/citum-core/commit/732f3e84a5a07e461546a5af56885e0779da8f27))

- Install candidate panic hook once ([`0b66cfd`](https://github.com/citum/citum-core/commit/0b66cfda813a98528a5e058c6bd16f506a2a29e6))

- Remove dead output-plan variants ([`2bab204`](https://github.com/citum/citum-core/commit/2bab2041a0bdfbc055ee4e09dc104088d91e4ba7))

- Unify scoring, dedupe tokenizer ([`d7133ba`](https://github.com/citum/citum-core/commit/d7133ba0f934bd0b7cd742a9c975f209eece18f0))

- Sparse-extract processing overrides ([`ae78468`](https://github.com/citum/citum-core/commit/ae78468265bdc15e17de168b88f5512983bd34a3))

- Introduce crate-level MigrateError ([`5022c2b`](https://github.com/citum/citum-core/commit/5022c2bfcdee2fe0f5f933df7e57ad607b420d61))


**schema**

- Add note-placement locale defaults ([`e6fdffd`](https://github.com/citum/citum-core/commit/e6fdffd37c4a60a20ef18fd4acc0ecad9b55847e))


**security**

- Bump crossbeam-epoch to 0.9.20 ([`b3627cb`](https://github.com/citum/citum-core/commit/b3627cb3117e59a6c97d3e55a6da349d5f77cb60))


**styles**

- Declare editor labels per csl oracle ([`a48067c`](https://github.com/citum/citum-core/commit/a48067c509681443df890f0b3fdc0eefb72fc5d1))



### Documentation

**migrate**

- Add crate review audit ([`68b8971`](https://github.com/citum/citum-core/commit/68b8971ae5a05a6d4d1199a9aeef194f0c61d7fa))



### Features

**engine**

- Gate substitute-title quoting ([`3af5ad5`](https://github.com/citum/citum-core/commit/3af5ad56cb980da797a257ca5fc53837c98fa998))

- Warn on unknown label term keys ([`2bf2bae`](https://github.com/citum/citum-core/commit/2bf2baeb5469e7ac3d6b28815388698fd44a3cf1))

- Warn on citation-number sort key ([`c402959`](https://github.com/citum/citum-core/commit/c402959382782671162d743910eda4a14c046f09))

- Remove implicit role auto-label ([`4185629`](https://github.com/citum/citum-core/commit/41856292812cd6f4f55f7880f3c803b882a92ec7))


**migrate**

- Emit custom processing base delta ([`2f1998f`](https://github.com/citum/citum-core/commit/2f1998f307029cf5df4fdf5f799aa117ecd834f9))


**schema**

- Add gated substitute title-quote ([`efc6edd`](https://github.com/citum/citum-core/commit/efc6edd88a36793da6fe81c6fb1fb1e3137cddac))

- Add processing base delta field ([`3051e60`](https://github.com/citum/citum-core/commit/3051e609176c1c17b421c2eee39fbdff350324b4))

- Add role-label defaults bundles ([`8cbd169`](https://github.com/citum/citum-core/commit/8cbd1693d0518b901845e90cef036d5d89ba802a))

- Add explicit role-label affixes ([`ea688fb`](https://github.com/citum/citum-core/commit/ea688fb6ebefc47127e3c1864ef17129ef3f3b6e))



### Performance

**engine**

- Unify Sorter into cached GroupSorter ([`cc97dff`](https://github.com/citum/citum-core/commit/cc97dfff9ee352f6c79667ff5c838eac19fc61a0))



### Refactor

**engine**

- Relocate sorter to crate root ([`3ef0107`](https://github.com/citum/citum-core/commit/3ef010712357ac126353620787722c4a8aa9e7ff))



### Testing

**engine**

- Lock in role-label default ([`d169b81`](https://github.com/citum/citum-core/commit/d169b81778b87003d2a4809ee118fca58565ed72))


## [0.72.0] - 2026-07-05

### Bug Fixes

**docs**

- Minor clarifications in code docs ([`4c38cab`](https://github.com/citum/citum-core/commit/4c38cabb624c73aff6a591f81f5a57772c0d62c4))


**engine**

- Resolve substitute/title-case gaps ([`95f5dea`](https://github.com/citum/citum-core/commit/95f5dea0b0cc3c99757db748450798c03cba5eca))

- Render chicago shared-corpus facts ([`8d37eac`](https://github.com/citum/citum-core/commit/8d37eac3dae72902270c1d1c64f6e925bfaf4380))

- Return error from process_document ([`466c041`](https://github.com/citum/citum-core/commit/466c041860590f9bdea3ba662e91342d8fd6c57b))

- Render group headings via format ([`d192bf5`](https://github.com/citum/citum-core/commit/d192bf54a778532a71e47122e07a7a97fe2ee578))

- Scan sub-spec templates for unknowns ([`f3844a7`](https://github.com/citum/citum-core/commit/f3844a754d50f08611816734b235b4683a3e98b0))

- Reject non-UTF-8 refs input files ([`ddae43f`](https://github.com/citum/citum-core/commit/ddae43ff41953d38be9f4e341b3bc6623bdfd534))

- Markdown offsets are body-relative ([`1f5fb10`](https://github.com/citum/citum-core/commit/1f5fb10dda547e5d0951c50182b58ced529630c7))

- Anchor frontmatter to line starts ([`98c39d8`](https://github.com/citum/citum-core/commit/98c39d8a31248ba8e419e3cda1000471c464e3f2))

- Escape html text output and data-ref ([`1d4eab7`](https://github.com/citum/citum-core/commit/1d4eab7b9e2b2c69086e3081bd490594309f3789))

- Preserve mixed-case words in casing ([`3418128`](https://github.com/citum/citum-core/commit/34181280d5b6a0e9a31a97bf5f0b3e4015b5a283))

- Honor delimiter in et-al joins ([`e0be49c`](https://github.com/citum/citum-core/commit/e0be49ca63cc808dce7071355f8aa19c34c290e3))

- Escape latex href targets ([`15e868e`](https://github.com/citum/citum-core/commit/15e868ed7f8efb1fe2f5fdaafda52df02fa5b833))

- Unify range formatting ([`d4f5ee1`](https://github.com/citum/citum-core/commit/d4f5ee122e1b7763833de48fe43933c8864b6d98))

- Type engine error variants ([`5cbc6d4`](https://github.com/citum/citum-core/commit/5cbc6d473525cad80b0c3351dca329c062367ff6))

- Format date ranges via single path ([`90e5239`](https://github.com/citum/citum-core/commit/90e52391665f53782629ccd3cf89315dadfe5c46))

- Centralize type classification ([`f8aff62`](https://github.com/citum/citum-core/commit/f8aff6274606af83bc960020a0b91f98d1cd6305))

- Map EDTF seasons to locale terms ([`47b6061`](https://github.com/citum/citum-core/commit/47b6061410fef5d20fa79183c3a9f764c5ef750a))

- Thread locale quote marks in render ([`530f396`](https://github.com/citum/citum-core/commit/530f396998f5e38f6809fad576755511db6223ae))


**locale**

- Form-aware general_term for no-date ([`a51bf9b`](https://github.com/citum/citum-core/commit/a51bf9b825c1720c164398146c48ed82662d5221))


**migrate**

- Validate note-field type override ([`2341762`](https://github.com/citum/citum-core/commit/2341762d410757774c368cedbe779c6f8c161e0b))


**schema**

- Merge partial locale raw maps ([`c855395`](https://github.com/citum/citum-core/commit/c855395b7d10d184c57902eac45abb5fe720705f))


**styles**

- Tune chicago author-date periodicals ([`bae9d2f`](https://github.com/citum/citum-core/commit/bae9d2fd70f9bbfa3b359970976b0f38d07a44ee))

- Tune chicago author-date fidelity ([`3ca2fad`](https://github.com/citum/citum-core/commit/3ca2fad1f717e62da513c9d48c659de249b6649b))

- Drop anchors and hardcoded text ([`c6f1dd3`](https://github.com/citum/citum-core/commit/c6f1dd3cf981502afe767cb3ba0fa880807fedca))

- Chicago author-date bib cluster lift ([`7146419`](https://github.com/citum/citum-core/commit/71464195d60153ac9e398f5ed75a5ee8442a7d0a))

- Wire chicago 18th reprint trailers ([`e83be2e`](https://github.com/citum/citum-core/commit/e83be2e06f4d9d8edfabfc62daa8e397a037fecf))

- Restore fidelity after locale switch ([`ba5d176`](https://github.com/citum/citum-core/commit/ba5d176020b2841282946d5bf2fbb67e923e4677))



### Documentation

**engine**

- Record 2026-07 crate review audit ([`ef85824`](https://github.com/citum/citum-core/commit/ef858248547ee4dbf8f912d83db9bff4441c8f93))

- Refresh stale crate layout notes ([`49e4787`](https://github.com/citum/citum-core/commit/49e4787f2e0043e187b9b3953c9dd0d1b12f9edf))

- Record part 2 crate review audit ([`b308389`](https://github.com/citum/citum-core/commit/b308389ac466c168c4f887058ee1d3667239ff35))

- Record part 2 triage dispositions ([`582fa28`](https://github.com/citum/citum-core/commit/582fa281688f5a9e4fd65d125b315d9e57f59c0f))

- Spec type-classification centralize ([`97858b4`](https://github.com/citum/citum-core/commit/97858b4d148123d4ba6ed4b3b29528ffbf6db2e1))

- Address Copilot review comments ([`32a01b9`](https://github.com/citum/citum-core/commit/32a01b9031c4e8f1e688f5f2c6c77786ce12ffed))


**report**

- Make report-core fidelity source ([`5acbae0`](https://github.com/citum/citum-core/commit/5acbae05fed13898b045daed76693a20c385189d))


**spec**

- Add csl type conversion contract ([`5bab1c6`](https://github.com/citum/citum-core/commit/5bab1c6f937d9775ce119990eed86770cb1afbc8))



### Features

**engine**

- Add localized type-label component ([`ff9763c`](https://github.com/citum/citum-core/commit/ff9763c4efea5673e2eb2cd52397de0a43e70c6a))


**report**

- Support citation-only oracle scope ([`a59fc85`](https://github.com/citum/citum-core/commit/a59fc85ef7f983b675a3ab925fe07d97f5b8c10d))

- Gate chicago-shared-corpus fidelity ([`75b61b1`](https://github.com/citum/citum-core/commit/75b61b1390915066d045c2195db32623f669ad73))


**schema**

- Close csl 1.0.2 type routing gaps ([`78078cf`](https://github.com/citum/citum-core/commit/78078cf530883469e1b31e057ec452fa4b93a846))

- Add original-publication conditions ([`60369f9`](https://github.com/citum/citum-core/commit/60369f9077d4139ce5785306cc3afbb00a66eadb))

- Add dates no-date-form style option ([`7f88e39`](https://github.com/citum/citum-core/commit/7f88e39b5199a92743e005d9b0b75233f51740cc))

- Gate anonymous-entry bib policy ([`888dafb`](https://github.com/citum/citum-core/commit/888dafba2163f3ce2f79ef426381a3c73debf4e1))

- Configure title delimiters ([`50f2aa5`](https://github.com/citum/citum-core/commit/50f2aa5992e8694714f1fb648a8062d36226ade4))


**styles**

- Add chicago-18-base component root ([`8532982`](https://github.com/citum/citum-core/commit/85329825479b2aa29285c2d60f64b107c7fab4bc))



### Performance

**engine**

- Use id stubs for custom-group path ([`9a45df9`](https://github.com/citum/citum-core/commit/9a45df94bc221c93d5b7e2de0bebe53464ad8542))

- Cache resolved session refs ([`e19b574`](https://github.com/citum/citum-core/commit/e19b5749fa77d31b9a570165c9ee07acda8e4bd9))

- Share render configs via Rc ([`80516f6`](https://github.com/citum/citum-core/commit/80516f6c9614a20ac32d822c7463bee2b7017c81))



### Refactor

**engine**

- Select disambiguation action ([`3e8fab7`](https://github.com/citum/citum-core/commit/3e8fab7ccd521c9f89605d09743d33034573502b))


**locale**

- Derive en-US from embedded YAML ([`96fa6c5`](https://github.com/citum/citum-core/commit/96fa6c53e1ef75cafaa5359e64193455ba8c5e97))



### Testing

**report**

- Update pinned APA concision score ([`284398c`](https://github.com/citum/citum-core/commit/284398ce8306f17db4d30229bea9626662e046a0))


## [0.71.0] - 2026-06-28

### Bug Fixes

**cli**

- Clean up render --help noise ([`1091096`](https://github.com/citum/citum-core/commit/109109669e5e36fc436655795dec8a195589ce72))


**scripts**

- Improve audit precision and recall ([`607fbbc`](https://github.com/citum/citum-core/commit/607fbbc587b8bcfc8691a987b1d81835e481ec41))



### Documentation

**docs**

- Add tune mode, embedded-tier gates ([`a29f815`](https://github.com/citum/citum-core/commit/a29f81506499380b52bca53aad504dd33b675fa4))


**locale**

- Add remaining phrase ids ([`d5f75d4`](https://github.com/citum/citum-core/commit/d5f75d4c2b99054357a8b42c129ed4f92876e395))

- Document message followups ([`85a2f18`](https://github.com/citum/citum-core/commit/85a2f189b11d827e86c3092193cd640062f87a0f))

- Track contributor phrase followup ([`cbaef1f`](https://github.com/citum/citum-core/commit/cbaef1fc7bd8fa28527eaa62c24c1ea04347e34b))

- Spec contributor phrase messages ([`1254427`](https://github.com/citum/citum-core/commit/1254427cecdade8121d50041af1214aae8ff753d))



### Features

**locale**

- Resolve term-backed message calls ([`7b3b9e0`](https://github.com/citum/citum-core/commit/7b3b9e0965b4c7952842ae9ae681e3f803905772))


**schema**

- Add localized message components ([`4afa50f`](https://github.com/citum/citum-core/commit/4afa50f77e3d604b9d6e7e39339674ba1bf8d88e))


**styles**

- Use embedded message phrases ([`5a0bf77`](https://github.com/citum/citum-core/commit/5a0bf77fac0e286e3545cee885decafe313b5f83))

- Convert apa chicago phrases ([`880701b`](https://github.com/citum/citum-core/commit/880701b8624c6eaa64071a11db9a69baedbbf772))



### Refactor

**cli**

- Extract style catalog row ([`58ef6f6`](https://github.com/citum/citum-core/commit/58ef6f655e0d91191732d462f674780ff2b1991e))


**engine**

- Simplify integral name memory ([`b2c1caf`](https://github.com/citum/citum-core/commit/b2c1caf19411643c3132f4919809d7ca57049639))

- Simplify substitute logic ([`41db0e9`](https://github.com/citum/citum-core/commit/41db0e93ba8279904ca57c6a49e679cb88ecc9ee))

- Simplify ungrouped rendering ([`58d3665`](https://github.com/citum/citum-core/commit/58d3665feaebc02b1ce91185de5a347d39ac2e85))


**locale**

- Split locale lookup modules ([`9112026`](https://github.com/citum/citum-core/commit/9112026fc56cf4509c37c0c8647de52c0b3d683f))


**migrate**

- Decouple template formatting ([`5fa5a49`](https://github.com/citum/citum-core/commit/5fa5a49489da6ea1ae950a12d3247c959a52a13e))


**schema**

- Rename issued date accessor ([`fa32312`](https://github.com/citum/citum-core/commit/fa323123a9b2bb4290775b20168bfce59bdfc260))


**styles**

- Migrate embedded terms ([`46c80c0`](https://github.com/citum/citum-core/commit/46c80c0794b269a4d128146c30036d5473cdf61c))

- Migrate production terms ([`842301c`](https://github.com/citum/citum-core/commit/842301cd636d6874bb6027d191483fa4871e6df0))



### Styling

**styles**

- Bring 6 styles to 100% fidelity ([`d3b5fe1`](https://github.com/citum/citum-core/commit/d3b5fe1080929603263b7e13cb93eeaf7e3f03a6))



### Testing

**engine**

- Clean document review smells ([`47537b0`](https://github.com/citum/citum-core/commit/47537b06c36a107d67a32b98496303ea1283ad52))


**report**

- Refresh top-10 oracle baseline ([`2de5f30`](https://github.com/citum/citum-core/commit/2de5f30798fd13dfe8e8e88569cf75b034716a87))


**styles**

- Forbid checked-in template terms ([`01b6eff`](https://github.com/citum/citum-core/commit/01b6eff7b23c7237f78f2d62f93eb2db9cd17889))


## [0.70.1] - 2026-06-23

### Bug Fixes

**ci**

- Add .cargo/audit.toml for audit ignores ([`d4354bf`](https://github.com/citum/citum-core/commit/d4354bf1faf3840bbadc4caf15238e38123a06b6))


**migrate**

- Raise numeric style fidelity floor ([`b85c13f`](https://github.com/citum/citum-core/commit/b85c13fe64c441470222bec444a03f8c24c93480))

- Refresh biblatex snapshots ([`79f1617`](https://github.com/citum/citum-core/commit/79f1617b3634c390cb45cf3bf08d926ed11673d8))


**security**

- Bump quinn-proto 0.11.15 ([`22afcc8`](https://github.com/citum/citum-core/commit/22afcc8df60afa8fbb022b4f62c833463fd4ee3c))


**styles**

- Raise ams-label fidelity to 93% ([`e864b4c`](https://github.com/citum/citum-core/commit/e864b4c273e78e6fa9c5b3ea0769c95c477f9147))



### Documentation

**demo**

- Generate from engine via just demo ([`2ca5df1`](https://github.com/citum/citum-core/commit/2ca5df1b70fffed00ec3e2cc86bf13db53234b32))

- Fix layout — drop duplicate h1 ([`b2e6748`](https://github.com/citum/citum-core/commit/b2e6748d29df9461c70691e00e2807a9f2e41c4f))

- Remove media query from sidebar layout ([`d0a93bd`](https://github.com/citum/citum-core/commit/d0a93bdcb1f2476e415fc2b73fabfedc97ee633d))

- Fix layout buttons, build guard ([`e0836f1`](https://github.com/citum/citum-core/commit/e0836f14320a2f6d4afc815b143c94cbc5fc42a8))



### Testing

**engine**

- Format_document extended tests ([`dfa5ad2`](https://github.com/citum/citum-core/commit/dfa5ad24726cf6659dc643d6f647d29011919d59))


## [0.70.0] - 2026-06-21

### Bug Fixes

**engine**

- Pull comma inside bibliography quotes ([`3d521ec`](https://github.com/citum/citum-core/commit/3d521ec0fb219c26ad6b5be4dd86d41513fccf07))

- Guide-conformant disambiguation ([`527db9d`](https://github.com/citum/citum-core/commit/527db9d88b85ead9094bbd9450f531ad00a43745))


**styles**

- Align core styles with guides ([`bc8043c`](https://github.com/citum/citum-core/commit/bc8043c2cf85355055ed015fb388a395bc0079ed))

- Structural conformance fixes ([`66588dd`](https://github.com/citum/citum-core/commit/66588ddd088c923e57f42d31965fcc3067e82b77))

- Apa/chicago author-date-full preset ([`3a56866`](https://github.com/citum/citum-core/commit/3a568662bc2f2f4524485577bcbca0163d2cce4f))



### Documentation

**report**

- Add style guide-conformance audit ([`fec1a1a`](https://github.com/citum/citum-core/commit/fec1a1a412a72fa402f132e472a3ce10a6852f6c))



### Features

**engine**

- Bounded render residual controls ([`2a7e6b2`](https://github.com/citum/citum-core/commit/2a7e6b2f98c390dd85fcc7d103526edebcef745a))

- Comma+short substitute editor label ([`f0d3724`](https://github.com/citum/citum-core/commit/f0d37240e3d53e84228f05c92108db70285a533f))


**schema**

- Text-case option for role labels ([`ebd3d9c`](https://github.com/citum/citum-core/commit/ebd3d9caa7b01b80bfcdb72baa77aca752fb4b8e))


## [0.69.0] - 2026-06-18

### Bug Fixes

**engine**

- Avoid doubling quotes on titles ([`24b6194`](https://github.com/citum/citum-core/commit/24b6194a1b4c82a003c3e351a30f3e0074d674eb))

- Suppress genre that echoes item type ([`c09bd77`](https://github.com/citum/citum-core/commit/c09bd77d7745ceb307d8a46b7409e654803e5020))


**migrate**

- Enforce regime on template links ([`eebd258`](https://github.com/citum/citum-core/commit/eebd2586e10828d801ee30bb9f42ef69f5817e82))

- Gate leaked in. type-variant term ([`0e6407d`](https://github.com/citum/citum-core/commit/0e6407d1e527fe0c88863bd03e5e33cef836abcc))

- Gate url/accessed to web types ([`291efe2`](https://github.com/citum/citum-core/commit/291efe25f8ed20ed6e205ebd793ed588d72ca42f))

- Preserve pre-wrapper type variants ([`f16bb1e`](https://github.com/citum/citum-core/commit/f16bb1e7ce3ca57cdad3aebd1a26c1424a8f0ff6))

- Improve authorable compression ([`26d2166`](https://github.com/citum/citum-core/commit/26d2166c688c632f5e24ff7cf8ed3e5c23d13602))

- Stdout for --json modes in analyze ([`a95cd0f`](https://github.com/citum/citum-core/commit/a95cd0f6e1ec40e0320fcd18b57a4fc13a664f9e))

- Correct CSL title semantics ([`9201f75`](https://github.com/citum/citum-core/commit/9201f7541ce5bbc13339461b5ad42836ed837da0))

- Use neutral CSL container title ([`f41630a`](https://github.com/citum/citum-core/commit/f41630a72ffc02316c4feff644d74503b1518936))

- Preserve CSL citation labels ([`30fdc34`](https://github.com/citum/citum-core/commit/30fdc346dc3bbf6a6b5779dcf64be2fa16fb2026))


**styles**

- Align RSC and AFS bibliographies ([`795f82c`](https://github.com/citum/citum-core/commit/795f82cd8af6c445267416ea6abaf7f076959ab8))



### Documentation

**migrate**

- Lock disambiguation presets ([`07500b6`](https://github.com/citum/citum-core/commit/07500b64f29a653fee212f6a26f1497fe85a2f36))

- Record order-aware fitness negative ([`2ac7229`](https://github.com/citum/citum-core/commit/2ac72297a8b84a68012a7ca85f2a979a8a98b113))

- Locus classification audit ([`86ff26a`](https://github.com/citum/citum-core/commit/86ff26a607208a86be9beeca35fca703ec5e5ee2))

- Defer dc1d/ya9b, vmcr in-progress ([`54f775c`](https://github.com/citum/citum-core/commit/54f775ca141c3791d90d38610d3c8e95c199ccc7))

- Xml compilation is a synthesis seed ([`eac79db`](https://github.com/citum/citum-core/commit/eac79dbdb9b2e8a72963ed21a6b10da1908e2651))

- Spec full-first type-variant arch ([`2958327`](https://github.com/citum/citum-core/commit/29583275650cb99740fd88001f474ebfe3637580))

- Add citum-analyze README ([`2facf2f`](https://github.com/citum/citum-core/commit/2facf2fb904a61bfd8f883d466e4dea4f88340c6))

- Update migration strategy analysis ([`014ccb5`](https://github.com/citum/citum-core/commit/014ccb5a4ead899877a75591c955cead043a6681))


**spec**

- Add synthesis loop phase two design ([`16afc2d`](https://github.com/citum/citum-core/commit/16afc2d475977a3fd811fe0cd62872843e977578))



### Features

**migrate**

- Positional citation scenarios ([`4a82966`](https://github.com/citum/citum-core/commit/4a829661586417b003e24220d80b64bc2ec3136b))

- Held-out fixture validation ([`b3b3124`](https://github.com/citum/citum-core/commit/b3b31242a5eb1ef51d6633070b70188bdfda42e3))

- Enforce candidate generation budget ([`1e85444`](https://github.com/citum/citum-core/commit/1e85444b065575e9c36ae883c579d97d9d28ec23))

- Add synthesis mutation operators ([`82980af`](https://github.com/citum/citum-core/commit/82980af19bed7fc4c43dfca0fab6c0afb9ff26ac))

- Add output-driven synthesis loop ([`1893619`](https://github.com/citum/citum-core/commit/18936194c33f3e6623d90fb2bdb6a490f532610c))

- Route migration through synthesis ([`c4e0dca`](https://github.com/citum/citum-core/commit/c4e0dca184a2cb6367b3eb52ebfe0a0dc3f279e1))

- Fold author-date presets ([`0ab5ca0`](https://github.com/citum/citum-core/commit/0ab5ca0858d1c1a30ec94b5ea5a69456b6b62c20))

- Render trigraph label styles ([`d17a88d`](https://github.com/citum/citum-core/commit/d17a88d8d72d085bf99fccab27c6be6999312aae))

- In-process batch-test pipeline ([`e04ef48`](https://github.com/citum/citum-core/commit/e04ef484da1ecdb4d28f8b023be332b3bdea41d8))

- Add --coverage-gap to citum-analyze ([`5750f90`](https://github.com/citum/citum-core/commit/5750f905ded0e641bc0175225d99a5467c0f782f))

- Add --config-presets mode ([`9b1156d`](https://github.com/citum/citum-core/commit/9b1156de789516f1b609288e4bbe7ea81dd1b93d))

- Expose measured selection evidence ([`5d1315e`](https://github.com/citum/citum-core/commit/5d1315e4af4d455961a811f99454ceacb2fe38a1))



### Refactor

**migrate**

- Split migration assembly ([`8db09ef`](https://github.com/citum/citum-core/commit/8db09efd152b2c9b12e2e53f92ae36c1344e2fa9))

- Split SQI refinement phase ([`2fcf7ac`](https://github.com/citum/citum-core/commit/2fcf7ac63f60fa2a73f625a73173821c69fb13c8))



### Testing

**engine**

- Cover author-date disambiguation ([`ec9233a`](https://github.com/citum/citum-core/commit/ec9233a709a281acbd19a752d9dd2d0fac9d5983))


**migrate**

- Harden SQI refinement guards ([`d9c3293`](https://github.com/citum/citum-core/commit/d9c3293a489df262f5ea7e780ca436fb8c1c073a))


## [0.68.0] - 2026-06-12

### Bug Fixes

**engine**

- Skip suppressed consumption ([`a989446`](https://github.com/citum/citum-core/commit/a989446dfc45d1e48ea0fdb6283dd0e7f32f18bf))


**migrate**

- Strip suppressed variable poison ([`93f8e85`](https://github.com/citum/citum-core/commit/93f8e8526b308901b3cc18eb0d1f918de2327e4b))

- Note-class citation repeat forms ([`5f3561e`](https://github.com/citum/citum-core/commit/5f3561ec1f4fa477bb2ff97cff391aae390e54bd))

- Full variants in wrapper emission ([`8c15f9d`](https://github.com/citum/citum-core/commit/8c15f9da948e01921581ca6eb6325bf99f2485b8))

- Keep default-branch bib order ([`a7afdbb`](https://github.com/citum/citum-core/commit/a7afdbbb3fed7a916d8d9b6b339bddca0ed50e08))



### Documentation

**spec**

- Test-soundness ledger + shared bar ([`f97c022`](https://github.com/citum/citum-core/commit/f97c022db9c86a73d9b76a3e9e33d578ea5f147e))

- Clarify two sorting silences ([`5113b25`](https://github.com/citum/citum-core/commit/5113b250f87c07154e6b29b7823534f52408b38a))

- Fill two multilingual spec silences ([`c36ded1`](https://github.com/citum/citum-core/commit/c36ded1d813f09cf3d66a6b805b5f7537cfda3be))

- Define consumption semantics ([`ed746ec`](https://github.com/citum/citum-core/commit/ed746ec0e8c4804bbf4292265a8889308a31cc57))



### Features

**i18n**

- Romanized-script-translated preset ([`4c6d2e5`](https://github.com/citum/citum-core/commit/4c6d2e544eee984cf8c3ff1fafc201cd7754a5fd))


**migrate**

- Random-corpus sqi scorecard mode ([`d28c6e9`](https://github.com/citum/citum-core/commit/d28c6e915a9a16ce2d6b392b04d4bc89f9446ef6))

- Measured citation selection ([`d296408`](https://github.com/citum/citum-core/commit/d296408f5d3c2f2a024b756592c0d4532257aea2))

- Add measured candidate selection ([`666cda1`](https://github.com/citum/citum-core/commit/666cda13979ee5a745ed782a0478ffdde0dd3e30))


**tooling**

- Rework test-soundness-review skill ([`6eb53df`](https://github.com/citum/citum-core/commit/6eb53dfadb0f5f0a4e99154001fe668ad1adee21))

- Auto-proceed after soundness audit ([`9624fe3`](https://github.com/citum/citum-core/commit/9624fe3533097a57ea2873626ce3280144a951e9))



### Refactor

**i18n**

- Original-script view + drop CNE ([`d6f2faf`](https://github.com/citum/citum-core/commit/d6f2faff6c66ef6f8b75031cbb038fd755aa3afe))



### Testing

**engine**

- Apply SORTING.md soundness findings ([`d2bbac0`](https://github.com/citum/citum-core/commit/d2bbac0e952fb3e958b8693133de16d9d022f24d))

- Harden disambiguation soundness ([`de54051`](https://github.com/citum/citum-core/commit/de54051f04b0916577956f2d1d1349f7cd97f3ea))

- Harden multilingual test soundness ([`28f2335`](https://github.com/citum/citum-core/commit/28f233502edbf2242593796ef3f21314fb973b13))


## [0.67.0] - 2026-06-09

### Bug Fixes

**engine**

- Warn on unknown reference fields ([`0016e96`](https://github.com/citum/citum-core/commit/0016e965d86c27f43895326784779445d80c2fa7))

- Render cne name patterns natively ([`7e2a709`](https://github.com/citum/citum-core/commit/7e2a70915d2c961b2f5087b2588c17608cbfad26))


**examples**

- Use Murakami instead of Murasaki ([`aab7d4d`](https://github.com/citum/citum-core/commit/aab7d4da0a88c0b887d5f1eb4974c4b95384bd0f))



### Documentation

**examples**

- Promote cne refs to examples/ ([`279b990`](https://github.com/citum/citum-core/commit/279b99058ad35cb4afccbfe816b58f04243fecba))



### Features

**i18n**

- Add cne chicago fixtures ([`5e3bbe0`](https://github.com/citum/citum-core/commit/5e3bbe0e9aa8a2a6495097a696e06824db8189a4))


## [0.66.0] - 2026-06-09

### Bug Fixes

**ci**

- Scope release notes to previous tag ([`558e6c0`](https://github.com/citum/citum-core/commit/558e6c06469dc0168cc76b6c791ebbe67c9c853e))


**engine**

- Same-author collapse first-class rule ([`1272383`](https://github.com/citum/citum-core/commit/12723834a1331de7af76abd8daf18beb91b7bb5b))



### Documentation

**spec**

- Reconcile bib grouping vocabulary ([`31e6daa`](https://github.com/citum/citum-core/commit/31e6daae5885cc84d71811422662b527513ce82d))

- Expand citation cluster rendering spec ([`9c7c4b2`](https://github.com/citum/citum-core/commit/9c7c4b22e24b84a623c3eac8f85273089eb5213f))



### Features

**edtf**

- Add FromStr and ParseError ([`7fccb7a`](https://github.com/citum/citum-core/commit/7fccb7ac327dfd488cb6fc8b0a824a530237011d))


**engine**

- Nocite bibliography-only entries ([`22212e9`](https://github.com/citum/citum-core/commit/22212e9d6046a5c882c91d29b9fd2899ea630b24))



### Refactor

**engine**

- Fold bib groups onto blocks path ([`3e75f63`](https://github.com/citum/citum-core/commit/3e75f63c0ddb9d0f35c05f6d2232f5e271f10b58))

- Unify document bib rendering ([`b5b01ba`](https://github.com/citum/citum-core/commit/b5b01bad751eccc5bc67f89713d896822b33acd7))


## [0.65.0] - 2026-06-08

### Features

**engine**

- Consolidate bib block rendering ([`fd0c6ee`](https://github.com/citum/citum-core/commit/fd0c6eee114d657a701eb5b5c1a72bc194940a83))


## [0.64.0] - 2026-06-07

### Bug Fixes

**engine**

- Honor preview citation mode ([`f86e5a7`](https://github.com/citum/citum-core/commit/f86e5a740d8af9ed204ea1a2f89e04c2cea086f2))

- Format bibliography entry html ([`80caaf6`](https://github.com/citum/citum-core/commit/80caaf6e58df707bb97809cde82076ed1f26a47b))

- Compose typst strong markup ([`52906b4`](https://github.com/citum/citum-core/commit/52906b43af6fef8b3f44aa7fc20f912e81d4b1f8))



### Documentation

**server**

- Add session API docs, fix examples ([`ce91379`](https://github.com/citum/citum-core/commit/ce91379f44cf30a4f4cabe8f677248edaf494785))


**tooling**

- Formalize repo-local harness ([`ff7cd2f`](https://github.com/citum/citum-core/commit/ff7cd2f71c720e93e505e4888d115c3f618f9ede))



### Features

**engine**

- Per-document style overrides ([`8d2f61e`](https://github.com/citum/citum-core/commit/8d2f61e495484b21b2d55652c03723505474506b))

- Wire name-memory through session API ([`7689814`](https://github.com/citum/citum-core/commit/7689814d30c03b09c85ea29e2c15eb47653dfe73))


## [0.63.0] - 2026-06-04

### Documentation

**spec**

- Activate per-doc overrides spec ([`d2c6d8b`](https://github.com/citum/citum-core/commit/d2c6d8b70540afc1ac2f3dfc006635d5cc4fdb91))



### Features

**schema**

- Multilingual presets + pattern mode ([`98ca8e4`](https://github.com/citum/citum-core/commit/98ca8e4cc66132f2dfbd15ca82d906be14e5d974))

- Citation sentence_start signal ([`acf6c29`](https://github.com/citum/citum-core/commit/acf6c29ffa418fc2ff4fc0f5eb30815baeb0968e))


**server**

- Add session api ([`ecea9dd`](https://github.com/citum/citum-core/commit/ecea9ddfaf5ac6d0be991ce7a4e2cbcc868aa878))



### Refactor

**engine**

- Clarify delimiter-join loops ([`baf9953`](https://github.com/citum/citum-core/commit/baf9953464e45f24c8d65cbde476e4e1d02d7c30))


## [0.62.0] - 2026-06-02

### Bug Fixes

**engine**

- Implement by-cite givenname expansion ([`b0fb8ca`](https://github.com/citum/citum-core/commit/b0fb8cac47bedaf9d0b84f4b80e9b0cfbc4a1558))

- Primary-name falls to suffix on tie ([`daf2748`](https://github.com/citum/citum-core/commit/daf2748fd7ce0d25cfc5392e0c9bf194b9376faa))


**styles**

- Chicago em-dash + cross-entry infra ([`b91dd61`](https://github.com/citum/citum-core/commit/b91dd616c22a25eef6391595437dd132934f8b01))



### Documentation

**spec**

- Cross-entry fidelity + givenname gap ([`c9b5565`](https://github.com/citum/citum-core/commit/c9b5565ffa9ba5406523f9fd331035dfde200c62))

- Align givenname-disambiguation-rule ([`03b9463`](https://github.com/citum/citum-core/commit/03b9463200f6994b1d7558413349c633aa2a4415))



### Features

**engine**

- Add biblatex refs input variant ([`b4e088d`](https://github.com/citum/citum-core/commit/b4e088de4498ec008f892a3358bbe040accb0386))


**schema**

- Givenname-disambiguation-rule ([`327252a`](https://github.com/citum/citum-core/commit/327252a0bf3d50b26a8effeb784e735e8a0453a9))



### Testing

**report**

- Regen oracle snapshots for fixtures ([`430dc6f`](https://github.com/citum/citum-core/commit/430dc6f49ee578f1ee4d8f13ac629ec72f3d5bb5))

- Refresh oracle baseline ([`9178e9c`](https://github.com/citum/citum-core/commit/9178e9c3a1652a741d40b671be2ddc0c302c80ca))


## [0.61.1] - 2026-06-01

### Bug Fixes

**docs**

- Update jsr package name ([`aff4854`](https://github.com/citum/citum-core/commit/aff4854081083173051e1ac7114a39c7ceadae82))

- Capabilties link ([`9f91a6b`](https://github.com/citum/citum-core/commit/9f91a6bdecf4bdb8cf77bb4f2270ac6763ec40b9))

- The other capabilities link ([`78ef5cf`](https://github.com/citum/citum-core/commit/78ef5cfe58dad2935177df0df989abb39b876543))

- Point to specs/ rather than arch ([`68e5685`](https://github.com/citum/citum-core/commit/68e56855119ddd772808ceab6c13ef7f55fe2abd))


**engine**

- Document bib restricted to cited refs ([`545328c`](https://github.com/citum/citum-core/commit/545328c52669a5ca7ae6cf341315439c23ceae87))



### Documentation

## [0.61.0] - 2026-06-01

### Bug Fixes

**engine**

- Honor grouped author-date delimiter ([`03f7be0`](https://github.com/citum/citum-core/commit/03f7be0680b17e01457ec67567764de0c2730cf8))

- Dedup across fenced-div bib blocks ([`9130123`](https://github.com/citum/citum-core/commit/9130123268a649ce6f52f9d7654dac3c1118c863))


**styles**

- Chicago author-date locator ([`e46afc4`](https://github.com/citum/citum-core/commit/e46afc4a0e6eddc106e925a398874bc5bc04d293))



### Documentation

**spec**

- Update markdown pandoc workflow ([`086db76`](https://github.com/citum/citum-core/commit/086db768bd0064f70b7623428012cdc7fb0c4a8d))

- Add capability index + SORTING.md ([`e3e258e`](https://github.com/citum/citum-core/commit/e3e258eec8029fc7a42d3fbb4af223d7df9a3ded))

- Fold completed rows + fix alint ([`fcf1e73`](https://github.com/citum/citum-core/commit/fcf1e73a30c27eefaf44b8b409ab7f413c3fc9c0))

- Fix 7 status mismatches + SORTING note ([`f3ed853`](https://github.com/citum/citum-core/commit/f3ed8539f82ed303bf1881b9ce3c09f716af2d94))

- Fix WASM + TEMPLATE_V3 vs reality ([`5080d78`](https://github.com/citum/citum-core/commit/5080d782ba96102805b6ac42bf1e209cc79dd8d1))



### Features

**engine**

- Add markdown output format ([`89b5142`](https://github.com/citum/citum-core/commit/89b5142ce02b817dc57936e866e1fcbabdbd913c))

- Markdown footnote placement ([`ec5110e`](https://github.com/citum/citum-core/commit/ec5110eabf2039767a3852b925722946d56bb413))

- Multilingual disambiguation key ([`3452db0`](https://github.com/citum/citum-core/commit/3452db03c7b51acd019a7716b6cff54c80f6fc65))

- Add format_document_with_resolver ([`8e2ce23`](https://github.com/citum/citum-core/commit/8e2ce23b9633649e4b7bebe2eb55eac27b2d7eee))



### Testing

**engine**

- Harden disambiguation test soundness ([`aa7af75`](https://github.com/citum/citum-core/commit/aa7af75828cd5459a2d9e4ea8c2bffff8f707071))


**locale**

- Add MaybeGendered snapshot coverage ([`787780d`](https://github.com/citum/citum-core/commit/787780d36d9fe6c923061bc62aaec3a0fade31cd))


## [0.60.0] - 2026-05-30

### Bug Fixes

**engine**

- Convert body markup for typst/latex ([`f278031`](https://github.com/citum/citum-core/commit/f2780312a20bddc92732f85b662709e281418a23))

- Fix inline code fences and escaping ([`d095c8f`](https://github.com/citum/citum-core/commit/d095c8f6d3d8722c6c54b1e5b49841c45af61bff))

- Convert markdown body markup to html ([`31b0503`](https://github.com/citum/citum-core/commit/31b05033476e14ca30459a603aa43c1b13b03954))


## [0.59.0] - 2026-05-29

### Bug Fixes

**ci**

- Add citum-refs to publish-crates order ([`0b7b654`](https://github.com/citum/citum-core/commit/0b7b654d80a9312075501bc7a09c9f35918dd439))


**demo**

- Style bibliography section headings ([`480b6c4`](https://github.com/citum/citum-core/commit/480b6c4f050dd5705e0589ab9e444111136d0f12))

- Split bib heading styles; enlarge h1 ([`e4df8d8`](https://github.com/citum/citum-core/commit/e4df8d86b304bf96d8012fab58da7e3f23ef2abc))

- Enlarge page title h1 to 2rem ([`59d5d1f`](https://github.com/citum/citum-core/commit/59d5d1f94baddd1d64c1244f048b7f913478d2c5))


**engine**

- Apply mode options to Renderer ([`c4c78e1`](https://github.com/citum/citum-core/commit/c4c78e16313e512e00c443150860035afed3a284))

- Resolve multilingual data-* attrs ([`e50702d`](https://github.com/citum/citum-core/commit/e50702dc1e190f6fb97a9086d6131105574c1e67))



### Documentation

**demo**

- Generate page from djot via engine ([`4d367e9`](https://github.com/citum/citum-core/commit/4d367e9f35ae029d2f6619cbc82e165614ebbc6e))


**spec**

- Consolidate disambiguation spec ([`3b7ac2f`](https://github.com/citum/citum-core/commit/3b7ac2fc9865bdf55245c61aab3b5e334a986278))



### Features

**engine**

- Year-suffix issued date only ([`94cb79d`](https://github.com/citum/citum-core/commit/94cb79d45391a458a987ed188be0aa21992fa25e))

- Suppress disambig title in xref ([`84b0070`](https://github.com/citum/citum-core/commit/84b00701d3d7c864580f885432c6ff39d0852f2d))


**schema**

- Add disambiguate.ignore option ([`73f4b70`](https://github.com/citum/citum-core/commit/73f4b70463846cd9195411e29aef00e30f05c6c0))



### Performance

**schema**

- Typed merge_style_overlay ([`98495bd`](https://github.com/citum/citum-core/commit/98495bd4d78909f9ef815e2d6e78c554159bc306))



### Refactor

**schema**

- Dry structural from deser impls ([`b54ba09`](https://github.com/citum/citum-core/commit/b54ba09bb05fdbc81b9e7d8591a312d47a9b40e6))


## [0.58.0] - 2026-05-27

### Bug Fixes

**ci**

- Explicit changelog generation step ([`a824da0`](https://github.com/citum/citum-core/commit/a824da0d5dc47836a3b7d4ed318f117f4e57d180))

- Replace dorny/paths-filter with git ([`d9ba15e`](https://github.com/citum/citum-core/commit/d9ba15e8d6e1daacccbc5f66e4cd70900cf3656f))

- Legal fixture path after spec dir move ([`f4d044f`](https://github.com/citum/citum-core/commit/f4d044fcb572d800ecf7c21347246af487dfba52))


**engine**

- Romanize multilingual APA titles ([`e931850`](https://github.com/citum/citum-core/commit/e931850c537f45ccb8e72b20c6317514800db8b9))

- Parse negated djot bibliography types ([`c9f9338`](https://github.com/citum/citum-core/commit/c9f9338ff1bfd1cbfad7d81beb5ba07067f84061))


**locale**

- Wire date forms to pattern.date-* ([`b6b6e75`](https://github.com/citum/citum-core/commit/b6b6e75a7a17dfa062fcc15b36792f124b357c5d))



### Documentation

**engine**

- Specify multilingual names ([`27f628a`](https://github.com/citum/citum-core/commit/27f628a80a67ded49dd16c754507306f18a29fd6))

- Per-document config overrides spec ([`3a287e8`](https://github.com/citum/citum-core/commit/3a287e8250e3403d15b14182657a705e1d717c73))



### Features

**cli**

- Add --locale flag to render doc ([`736201b`](https://github.com/citum/citum-core/commit/736201b30e324411ed7ce4d49fcdf7b54b38f37f))


**engine**

- Support script name separators ([`26f0159`](https://github.com/citum/citum-core/commit/26f01593210781a87088f8614e38739dfa427c78))

- Split name-memory configs ([`3d1495b`](https://github.com/citum/citum-core/commit/3d1495b77d3f5df0aa9c98e31a5e3a1d5d52602d))

- Document options override structs ([`4d2ebf7`](https://github.com/citum/citum-core/commit/4d2ebf7e967942a6ac74eea25c121ca2f2729711))

- Wire document options into pipeline ([`004bd37`](https://github.com/citum/citum-core/commit/004bd374118f16412e40b280ab24b088f4c6021b))



### Build

**tooling**

- Add wasm-release cargo profile ([`73ffad8`](https://github.com/citum/citum-core/commit/73ffad8b16838eb08f79ee40d49f522d26366b98))


## [0.57.0] - 2026-05-25

### Bug Fixes

**release**

- Align schema changelog ([`3ad75c3`](https://github.com/citum/citum-core/commit/3ad75c3504ca616029ebb1dfce8e107ac8617533))



### Documentation

**release**

- Add jsr package readme ([`e72f343`](https://github.com/citum/citum-core/commit/e72f343100b02be74ad7b2f9dd51a96d123bcbd1))


## [0.42.0] - 2026-05-13

### Bug Fixes

**ci**

- Action versions and clippy hygiene ([`544ecd6`](https://github.com/citum/citum-core/commit/544ecd6d02454a885845c6dcf1f78dc0d5c186e8))


## [0.39.0] - 2026-05-11

### Features

**bindings**

- Add WASM performance benchmark ([`e187def`](https://github.com/citum/citum-core/commit/e187def85a248521a1ea6a041c43083aeb9aa5ea))


**schema**

- Rename template reuse ([`df4c4ea`](https://github.com/citum/citum-core/commit/df4c4eaaa6e7b69486634a47a1ed355eb22a82ed))


**server**

- Wire format_document arm + wasm ([`4b7dc22`](https://github.com/citum/citum-core/commit/4b7dc22834eda499ad8e9920de3c68bee4ae5a35))


## [0.35.0] - 2026-05-03

### Features

**cli**

- Distributed architecture phase 1 ([`7ae000d`](https://github.com/citum/citum-core/commit/7ae000d5d4ed3764e52634f954e4633f7f58d3bb))


## [0.34.0] - 2026-05-03

### Features

**schema**

- Rename template reuse ([`c95ddba`](https://github.com/citum/citum-core/commit/c95ddba8b5463f5aa8109b7dfb56dbc2b8ca992f))


## [0.30.2] - 2026-04-30

### Refactor

**schema**

- Strengthen ref and language ids ([`08f7fdc`](https://github.com/citum/citum-core/commit/08f7fdc0929d92fd7e997ecb6aaeb4f5785de11e))


**styles**

- Consolidate embedded style files ([`1f0e513`](https://github.com/citum/citum-core/commit/1f0e513c692a2a4caacc10b10520374303277b2b))


## [0.18.0] - 2026-03-25

### Bug Fixes

**lint**

- Pedantic autofixes ([`b52e2b0`](https://github.com/citum/citum-core/commit/b52e2b094f2628cb9df5d9bd3f34155f58f70a22))


**schema**

- Null-aware preset overlay merging ([`080df92`](https://github.com/citum/citum-core/commit/080df92f2efa1e980935bbb2ae92f2ffffeb8267))



### Features

**bindings**

- Add citum-bindings crate ([`c149b68`](https://github.com/citum/citum-core/commit/c149b685a0c4390c94c02e951ea23f3f7305d1e8))

- Promote wasm api + specta types ([`50376ae`](https://github.com/citum/citum-core/commit/50376ae1a244db83af72596bd240ce72435c2712))


**lint**

- Clippy all=deny, pedantic suppressions ([`9170a21`](https://github.com/citum/citum-core/commit/9170a2197df4c289e1202cfed840c8450e2b927c))


