# Hype cheat and internal-input catalog

Captured 2026-09-12 from the installed English PC build. This is a catalog of **272 code bindings**, not a claim that all 272 work. The input table has 497 actions in total. See [hidden lever investigation](../../notes/hype-cheat-exploration-2026-09-12.md) for activation, measured results and limitations.

**Evidence labels:** `AI` means a matching typed input reference was found in the loaded manor/Fix AI snapshot; `binding only` means no such reference was found there. A binding may be handled by native code, another level, or abandoned development code. Neither label alone proves a runtime effect. Names below are the game’s own action labels, with the common `Button_` prefix removed for readability.

The lever gates `tele01`–`tele36`, `hunter`, and `geritole` in the cheat controller. The other bindings are not automatically enabled by that lever. The two documented controls outside this text-code catalog are `Space` for lever interaction and the script-confirmed `Left Shift + Left Ctrl + Delete` bulk quest-flag shortcut; see the report before using the latter.

## Lever and teleports

| Code | Engine action | Snapshot evidence |
| --- | --- | --- |
| `debug` | `DebugTeleport` | AI |
| `tele01` | `Teleport01` | AI |
| `tele02` | `Teleport02` | AI |
| `tele03` | `Teleport03` | AI |
| `tele04` | `Teleport04` | AI |
| `tele05` | `Teleport05` | AI |
| `tele06` | `Teleport06` | AI |
| `tele07` | `Teleport07` | AI |
| `tele08` | `Teleport08` | AI |
| `tele09` | `Teleport09` | AI |
| `tele10` | `Teleport10` | AI |
| `tele11` | `Teleport11` | AI |
| `tele12` | `Teleport12` | AI |
| `tele13` | `Teleport13` | AI |
| `tele14` | `Teleport14` | AI |
| `tele15` | `Teleport15` | AI |
| `tele16` | `Teleport16` | AI |
| `tele17` | `Teleport17` | AI |
| `tele18` | `Teleport18` | AI |
| `tele19` | `Teleport19` | AI |
| `tele20` | `Teleport20` | AI |
| `tele21` | `Teleport21` | AI |
| `tele22` | `Teleport22` | AI |
| `tele23` | `Teleport23` | AI |
| `tele24` | `Teleport24` | AI |
| `tele25` | `Teleport25` | AI |
| `tele26` | `Teleport26` | AI |
| `tele27` | `Teleport27` | AI |
| `tele28` | `Teleport28` | AI |
| `tele29` | `Teleport29` | AI |
| `tele30` | `Teleport30` | AI |
| `tele31` | `Teleport31` | AI |
| `tele32` | `Teleport32` | AI |
| `tele33` | `Teleport33` | AI |
| `tele34` | `Teleport34` | AI |
| `tele35` | `Teleport35` | AI |
| `tele36` | `Teleport36` | AI |

## Inventory bindings

| Code | Engine action | Snapshot evidence |
| --- | --- | --- |
| `ite00` | `CheatInv_Prison_Key` | binding only |
| `ite01` | `CheatInv_Gogoud_Key` | binding only |
| `ite02` | `CheatInv_BlackTower_Key` | binding only |
| `ite03` | `CheatInv_Chest_Key` | binding only |
| `ite04` | `CheatInv_Treasure_Key` | binding only |
| `ite05` | `CheatInv_Almanach_Monastere` | binding only |
| `ite06` | `CheatInv_Almanach_Enost` | binding only |
| `ite07` | `CheatInv_Almanach_Laboratory` | binding only |
| `ite08` | `CheatInv_Grimoire` | binding only |
| `ite09` | `CheatInv_Collier_Zatila` | binding only |
| `ite10` | `CheatInv_Bracelet` | binding only |
| `ite11` | `CheatInv_Thieves_Badge` | binding only |
| `ite12` | `CheatInv_VIP_Card` | binding only |
| `pouletfrit` | `CheatInv_Galarneau_Ad` | AI |
| `ite14` | `CheatInv_Senekal_Hammer_Helmet` | binding only |
| `ite15` | `CheatInv_Yellow_Flag` | binding only |
| `ite16` | `CheatInv_Blue_Flag` | binding only |
| `ite17` | `CheatInv_Red_Flag` | binding only |
| `ite18` | `CheatInv_Purple_flag` | binding only |
| `ite19` | `CheatInv_Symbol_1` | binding only |
| `ite20` | `CheatInv_Symbol_2` | binding only |
| `ite21` | `CheatInv_Symbol_3` | binding only |
| `ite22` | `CheatInv_Symbol_1_2` | binding only |
| `ite23` | `CheatInv_Symbol_1_3` | binding only |
| `ite24` | `CheatInv_Symbol_2_3` | binding only |
| `ite25` | `CheatInv_Symbol_1_2_3` | binding only |
| `ite26` | `CheatInv_Monastere_Key` | binding only |

## Story and quest bindings

| Code | Engine action | Snapshot evidence |
| --- | --- | --- |
| `gl000` | `Cheat_bhypepossedeepee` | binding only |
| `gl001` | `Cheat_bhypepossedearbalete` | binding only |
| `gl002` | `Cheat_bhypenepassedeplusrien` | binding only |
| `gl003` | `Cheat_bhypearriveduntimetravel` | binding only |
| `gl004` | `Cheat_bhypeestenarmure` | binding only |
| `gl021` | `Cheat_bsequence_auberge_fini_torras` | binding only |
| `gl035` | `Cheat_bliberertalboth` | binding only |
| `gl036` | `Cheat_bhypeattackzatila` | binding only |
| `gl037` | `Cheat_bhypeattackzatila2` | binding only |
| `gl038` | `Cheat_bhypedonneplastron` | binding only |
| `gl039` | `Cheat_bvolzatila` | binding only |
| `gl040` | `Cheat_bgogouddirecharger` | binding only |
| `gl041` | `Cheat_bliberergogoud` | binding only |
| `gl042` | `Cheat_brencontrerkaron` | binding only |
| `gl043` | `Cheat_bdonnerbraceletbolduk` | binding only |
| `gl044` | `Cheat_bautorisationpdb` | binding only |
| `gl045` | `Cheat_bsenekalvoyagetemps` | binding only |
| `gl046` | `Cheat_bsenekalarmes` | binding only |
| `gl047` | `Cheat_bsenekalarmesdonnees` | binding only |
| `gl048` | `Cheat_bgogoudparlecite` | binding only |
| `gl049` | `Cheat_bgogoudalmanach` | binding only |
| `gl050` | `Cheat_bravealmanach` | binding only |
| `gl0510` | `Cheat_enombrealmanach0` | binding only |
| `gl0511` | `Cheat_enombrealmanach1` | binding only |
| `gl0512` | `Cheat_enombrealmanach2` | binding only |
| `gl0513` | `Cheat_enombrealmanach3` | binding only |
| `gl052` | `Cheat_balmanachgogouddonnes3` | binding only |
| `gl053` | `Cheat_bvibedonnecle` | binding only |
| `gl054` | `Cheat_bgogoudfantome` | binding only |
| `gl055` | `Cheat_bdraregparle` | binding only |
| `gl0561` | `Cheat_enombreetendard1` | binding only |
| `gl0562` | `Cheat_enombreetendard2` | binding only |
| `gl0563` | `Cheat_enombreetendard3` | binding only |
| `gl0564` | `Cheat_enombreetendard4` | binding only |
| `gl0560` | `Cheat_enombreetendard0` | binding only |
| `gl057` | `Cheat_bvoyagerate` | binding only |
| `gl060` | `Cheat_bmonast3_clochesonnee` | binding only |
| `gl061` | `Cheat_bmonast3_possedealmanach` | binding only |
| `gl062` | `Cheat_bmonast2_possedeetendard` | binding only |
| `gl063` | `Cheat_b_isumo_mort` | binding only |
| `gl064` | `Cheat_bcachotsfini` | binding only |
| `gl065` | `Cheat_bpossedealmanachtour` | binding only |
| `gl066` | `Cheat_bgladiateurvaincu` | binding only |
| `gl067` | `Cheat_bmanoir3_aquitteapresavoirlibererave` | binding only |
| `gl068` | `Cheat_bhype_estdanslemanoir_mad` | binding only |
| `gl069` | `Cheat_bestdansmanoir` | binding only |
| `gl071` | `Cheat_bjoyau2_fun` | binding only |
| `gl072` | `Cheat_betendard_casino` | binding only |
| `gl073` | `Cheat_bintroduction_terminer_manoir` | binding only |
| `gl074` | `Cheat_balmanach_labo3_fini_mad` | binding only |
| `gl075` | `Cheat_bpossede_magieglace_labo_mad` | binding only |
| `gl076` | `Cheat_brave_before` | binding only |
| `gl077` | `Cheat_brave_after` | binding only |
| `gl078` | `Cheat_btheifrock_spoken` | binding only |
| `gl079` | `Cheat_bnohlin_before` | binding only |
| `gl080` | `Cheat_bnohlin_after` | binding only |
| `gl081` | `Cheat_bmessanger_theif` | binding only |
| `gl082` | `Cheat_bdejat_karon` | binding only |
| `gl083` | `Cheat_bnohlin_v4_parle` | binding only |
| `gl084` | `Cheat_bnohlin_c` | binding only |
| `gl085` | `Cheat_banoth_parle` | binding only |
| `gl086` | `Cheat_bexar_parle` | binding only |
| `gl087` | `Cheat_bgalnoth_parle` | binding only |
| `gl088` | `Cheat_bzatila_parle` | binding only |
| `gl089` | `Cheat_bzatila3_speak_1` | binding only |
| `gl090` | `Cheat_bzatila3_speak_2` | binding only |
| `gl091` | `Cheat_bzatila3_speak_3` | binding only |
| `gl092` | `Cheat_bzatila3_speak_4` | binding only |
| `gl093` | `Cheat_bzatila3_speak_5` | binding only |
| `gl094` | `Cheat_bzatila3_speak_6` | binding only |
| `gl095` | `Cheat_balmanachtour` | binding only |
| `gl096` | `Cheat_bkaron_village2` | binding only |
| `gl097` | `Cheat_bkaron_village2_b` | binding only |
| `gl098` | `Cheat_bkaron_village2_c` | binding only |
| `gl099` | `Cheat_bturoth_village2` | binding only |
| `gl100` | `Cheat_bmodor_village2` | binding only |
| `gl101` | `Cheat_bterryo_village2` | binding only |
| `gl102` | `Cheat_bterryo_karon` | binding only |
| `gl103` | `Cheat_bendoftheifmovie` | binding only |
| `gl104` | `Cheat_bhypereturnsbracelet` | binding only |
| `gl105` | `Cheat_bzatila_4a` | binding only |
| `gl106` | `Cheat_bzatila_4b` | binding only |
| `gl107` | `Cheat_bzatila_4c` | binding only |
| `gl108` | `Cheat_bzatila_4d` | binding only |
| `gl109` | `Cheat_bhypezattueveribadciel` | binding only |
| `gl110` | `Cheat_bhypezatsortdetowerload` | binding only |
| `gl111` | `Cheat_bsequence_rajoth_fini_mad` | binding only |
| `gl112` | `Cheat_bpremierefoisquehypezatdansleciel` | binding only |
| `gl113` | `Cheat_bpremierefoisquehypezatdansleciel1` | binding only |
| `gl114` | `Cheat_bpremierefoisquehypezatdansleciel2` | binding only |
| `gl115` | `Cheat_bpremierefoisquehypezatdansleciel3` | binding only |
| `gl116` | `Cheat_bpremierefoisquehypezatdansleciel4` | binding only |
| `gl117` | `Cheat_broilibere` | binding only |
| `gl118` | `Cheat_bfilleastronaute` | binding only |
| `gl119` | `Cheat_bgardeattaque_detoras2` | binding only |
| `gl120` | `Cheat_bGranslackisdead` | binding only |
| `gl121` | `Cheat_bhypeaparleauroidefort` | binding only |
| `gl122` | `Cheat_bEstAllerDansVillage` | binding only |
| `gl123` | `Cheat_bsequence_auberge_fini_torras2` | binding only |
| `gl124` | `Cheat_bsequence_auberge_fini_torras3` | binding only |
| `gl125` | `Cheat_bsequence_auberge_fini_torras4` | binding only |
| `gl126` | `Cheat_bHypeAParleAuRoiDeFort3` | binding only |
| `gl127` | `Cheat_bHypeAparleAuPrince` | binding only |
| `gl128` | `Cheat_bHypeALibereTaskan3` | binding only |
| `gl129` | `Cheat_bHypeAparleAGogoudFantome` | binding only |
| `gl130` | `Cheat_bLibererNathaniel` | binding only |
| `gl131` | `Cheat_bAlerteManoir2` | binding only |
| `gl132` | `Cheat_bfemmemenage_toras2` | binding only |
| `gl133` | `Cheat_bAstronauteHasLanded` | binding only |
| `gl134` | `Cheat_bIntroductionTerminee_Manoir_MAD` | binding only |
| `gl135` | `Cheat_bGogood_AttendLeJoyeau1_Manoir_MAD` | binding only |
| `gl136` | `Cheat_bAlchimiste1_FreeAsABird_MAD` | binding only |
| `gl137` | `Cheat_bPortePrison_Alchimiste1_Ouverte_MAD` | binding only |
| `gl138` | `Cheat_bAlchimiste2_JoueLesFillesDeLAir_MAD` | binding only |
| `gl139` | `Cheat_bPortePrison_Alchimiste2_Ouverte_MAD` | binding only |
| `gl140` | `Cheat_bAlchimiste3_SeFaitLaMalle_MAD` | binding only |
| `gl141` | `Cheat_bPortePrison_Alchimiste3_Ouverte_MAD` | binding only |
| `gl142` | `Cheat_bPorteVapeur_Ouverte_MAD` | binding only |
| `gl143` | `Cheat_bHypeEstPrisonnierDesLabo_MAD` | binding only |
| `gl144` | `Cheat_bCritalDeRajoth_Detruit_MAD` | binding only |
| `gl145` | `Cheat_bGogoodEnPrison_AParlee1FoisAHype_MAD` | binding only |
| `gl146` | `Cheat_bDestruction_MurDeFeu_Labo_MAD` | binding only |
| `gl147` | `Cheat_bDestruction_MurDeFeu_Antichambre_MAD` | binding only |
| `gl148` | `Cheat_bGogood_FreeAsABird_Labo_MAD` | binding only |
| `gl149` | `Cheat_bGogood_FreeAsABird_Antichambre_MAD` | binding only |
| `gl150` | `Cheat_bGrilleAntichambre_Ouverte_MAD` | binding only |
| `gl151` | `Cheat_bGogood_AttentSortieDeHype_MAD` | binding only |
| `gl152` | `Cheat_bLaboEstTerminee_Fini_OnEnParlePlus_MAD` | binding only |
| `gl153` | `Cheat_bRajothMangeLesPissenlitsParLaRacine_MAD` | binding only |
| `gl154` | `Cheat_bGrilleAuqueduc_Ouverte_MAD` | binding only |
| `gl155` | `Cheat_bTapFlag1_APassezLarmeAGauche_MAD` | binding only |
| `gl156` | `Cheat_bTapFlag2_APassezLarmeAGauche_MAD` | binding only |
| `gl157` | `Cheat_bFabriquePotion_Terminee_MAD` | binding only |
| `gl158` | `Cheat_bPossedeClefDesLaboratoire_MAD` | binding only |
| `gl159` | `Cheat_bPorteAntichambre_Ouverte_MAD` | binding only |
| `gl160` | `Cheat_bGrillePourEvasion_Ouverte_MAD` | binding only |
| `gl161` | `Cheat_bFaireTournerTuyau4_Terminee_MAD` | binding only |
| `gl162` | `Cheat_bPorteFoyer_Ouverte_MAD` | binding only |
| `gl163` | `Cheat_bChanceDeDonnerPotion_Terminee_MAD` | binding only |
| `gl164` | `Cheat_bEnigme_Planetes2_Terminee_MAD` | binding only |
| `gl165` | `Cheat_bManoir3_AQuitteAvantAvoirLibereRave` | binding only |
| `gl166` | `Cheat_bTheifAlreadyStolen` | binding only |
| `gl167` | `Cheat_bManoir3_ApresVoyageRate` | binding only |
| `gl168` | `Cheat_b_brigand2_Zatila_Counter_1` | binding only |
| `gl169` | `Cheat_b_brigand2_Zatila_Counter_2` | binding only |
| `gl170` | `Cheat_HypeSeTrouveDansLaSalleDesMannequins_MAD` | binding only |
| `gl171` | `Cheat_bTalbothParle` | binding only |
| `gl172` | `Cheat_bKaron_Talk_Counter1` | binding only |
| `gl173` | `Cheat_bMonast1_TalbothCinematik` | binding only |
| `gl174` | `Cheat_bGrillePont_Ouverte_Manoir_MAD` | binding only |
| `gl175` | `Cheat_bHypeEstDansLesAqueducs_MAD` | binding only |
| `gl176` | `Cheat_bzatila_parle2` | binding only |

## Other bindings

| Code | Engine action | Snapshot evidence |
| --- | --- | --- |
| `supered` | `SuperEd` | binding only |
| `allpowers` | `ModeAllPowers` | binding only |
| `fulllife` | `ModeFullLife` | binding only |
| `p1` | `ModePouvoir1` | binding only |
| `p2` | `ModePouvoir2` | binding only |
| `p3` | `ModePouvoir3` | binding only |
| `p4` | `ModePouvoir4` | binding only |
| `p5` | `ModePouvoir5` | binding only |
| `p6` | `ModePouvoir6` | binding only |
| `p0` | `ModePasDePouvoir` | binding only |
| `pa` | `ModePouvoirAbeilles` | binding only |
| `shad` | `ShadowOnActor` | binding only |
| `light` | `ChangeNegativeLight` | binding only |
| `fps` | `ShowFps` | binding only |
| `krok` | `ResetCamera` | binding only |
| `firehed` | `Cheat_Hypetetefeu` | binding only |
| `kutkut` | `Cheat_Hypehache` | AI |
| `wonderful` | `Cheat_SuperCourse` | binding only |
| `toutte` | `Cheat_Tout` | binding only |
| `rotek` | `Cheat_MegaArmure` | binding only |
| `tunnel` | `Cheat_CameraFocal` | AI |
| `thereyougo` | `Cheat_ToutesMagies` | AI |
| `toutundefi` | `Cheat_DetruireArmure` | AI |
| `hermetik` | `Cheat_InfiniMagie` | AI |
| `houdini` | `Cheat_InfiniFleche` | AI |
| `louloutre` | `Cheat_FlecheCam` | binding only |
| `epok1` | `Cheat_Epoque1` | binding only |
| `epok2` | `Cheat_Epoque2` | binding only |
| `epok3` | `Cheat_Epoque3` | binding only |
| `epok4` | `Cheat_Epoque4` | binding only |
| `grolot` | `Cheat_UnDeTout` | AI |
| `oint` | `Cheat_TakePosResetMap` | AI |
| `revient` | `Cheat_ResetMapButNotHype` | AI |
| `youngelf` | `Cheat_FlechesBleues` | AI |
| `oldelf` | `Cheat_FlechesRouges` | AI |
| `nrg1` | `Cheat_Jewel1_Charge` | binding only |
| `nrg2` | `Cheat_Jewel2_Charge` | binding only |
| `nrg3` | `Cheat_Jewel3_Charge` | binding only |
| `nrg4` | `Cheat_Jewel4_Charge` | binding only |
| `nonrg` | `Cheat_JewelDechargeTous` | binding only |
| `joyo1` | `Cheat_GetJewel1` | binding only |
| `joyo2` | `Cheat_GetJewel2` | binding only |
| `joyo3` | `Cheat_GetJewel3` | binding only |
| `joyo4` | `Cheat_GetJewel4` | binding only |
| `joyo0` | `Cheat_GetJewel0` | binding only |
| `frik` | `Cheat_Plastycks` | AI |
| `littletroll` | `Cheat_Regeneration` | AI |
| `littlegogoud` | `Cheat_MagieRegen` | AI |
| `thundergod` | `Cheat_Invincible` | binding only |
| `geritole` | `Cheat_PauseBall` | AI |
| `littleelf` | `Cheat_FlecheRebond` | binding only |
| `protek` | `Cheat_FullArmor` | AI |
| `druidik` | `Cheat_FullMagic` | AI |
| `leben` | `Cheat_FullLife` | AI |
| `hunter` | `Cheat_HomingBalls` | AI |
| `lod` | `Test_Loadgame` | binding only |
