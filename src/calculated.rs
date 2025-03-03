//! This module handles the formatting of the calculations (`The factorial of Subfactorial of 5 is`, etc.)
use crate::math::{FLOAT_PRECISION, LN10};
use crate::pending::TOO_BIG_NUMBER;
use crate::reddit_comment::{NUMBER_DECIMALS_SCIENTIFIC, PLACEHOLDER};

use rug::float::OrdFloat;
use rug::ops::Pow;
use rug::{Float, Integer};
use std::fmt::Write;

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub(crate) enum CalculatedFactorial {
    Exact(Integer),
    Approximate(OrdFloat, Integer),
    ApproximateDigits(Integer),
}

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub(crate) struct Factorial {
    pub(crate) number: Integer,
    pub(crate) levels: Vec<i32>,
    pub(crate) factorial: CalculatedFactorial,
}

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub(crate) struct Gamma {
    pub(crate) number: OrdFloat,
    pub(crate) gamma: OrdFloat,
}

#[derive(Debug, Clone, PartialEq, Ord, Eq, Hash, PartialOrd)]
pub(crate) enum Calculation {
    Factorial(Factorial),
    Gamma(Gamma),
}

impl Calculation {
    pub(crate) fn format(
        &self,
        acc: &mut String,
        force_shorten: bool,
    ) -> Result<(), std::fmt::Error> {
        match self {
            Self::Factorial(fact) => fact.format(acc, force_shorten),
            Self::Gamma(gamma) => gamma.format(acc),
        }
    }
    pub(crate) fn is_aproximate_digits(&self) -> bool {
        matches!(
            self,
            Calculation::Factorial(Factorial {
                factorial: CalculatedFactorial::ApproximateDigits(_),
                ..
            })
        )
    }
    pub(crate) fn is_approximate(&self) -> bool {
        matches!(
            self,
            Calculation::Factorial(Factorial {
                factorial: CalculatedFactorial::Approximate(_, _),
                ..
            })
        )
    }
    pub(crate) fn is_too_long(&self) -> bool {
        match self {
            Self::Factorial(fact) => fact.is_too_long(),
            Self::Gamma(_) => false,
        }
    }
}

impl Factorial {
    pub(crate) fn format(
        &self,
        acc: &mut String,
        force_shorten: bool,
    ) -> Result<(), std::fmt::Error> {
        let factorial_string = self.levels.iter().rev().fold(String::new(), |a, e| {
            format!(
                "{}{}{}",
                a,
                Self::get_factorial_level_string(*e),
                PLACEHOLDER
            )
        });
        match &self.factorial {
            CalculatedFactorial::Exact(factorial) => {
                let factorial = if self.is_too_long() || force_shorten {
                    Self::truncate(factorial, true)
                } else {
                    factorial.to_string()
                };
                write!(
                    acc,
                    "{}{} is {} \n\n",
                    factorial_string, self.number, factorial
                )
            }
            CalculatedFactorial::Approximate(base, exponent) => {
                let (base, exponent) = (base.as_float().clone(), exponent.clone());
                let exponent = if self.is_too_long() || force_shorten {
                    format!("({})", Self::truncate(&exponent, false))
                } else {
                    exponent.to_string()
                };
                let number = if self.number > *TOO_BIG_NUMBER || force_shorten {
                    Self::truncate(&self.number, false)
                } else {
                    self.number.to_string()
                };
                let base = base.to_f64();
                write!(
                    acc,
                    "{}{} is approximately {} × 10^{} \n\n",
                    factorial_string, number, base, exponent
                )
            }
            CalculatedFactorial::ApproximateDigits(digits) => {
                let digits = if self.is_too_long() || force_shorten {
                    Self::truncate(digits, false)
                } else {
                    digits.to_string()
                };
                let number = if self.number > *TOO_BIG_NUMBER || force_shorten {
                    Self::truncate(&self.number, false)
                } else {
                    self.number.to_string()
                };
                write!(
                    acc,
                    "{}{} has approximately {} digits \n\n",
                    factorial_string, number, digits
                )
            }
        }
    }

    fn truncate(number: &Integer, add_roughly: bool) -> String {
        let length = (Float::with_val(FLOAT_PRECISION, number).ln() / &*LN10)
            .to_integer_round(rug::float::Round::Down)
            .unwrap()
            .0;
        let truncated_number: Integer = number
            / (Float::with_val(FLOAT_PRECISION, 10)
                .pow((length.clone() - NUMBER_DECIMALS_SCIENTIFIC - 1u8).max(Integer::ZERO))
                .to_integer()
                .unwrap());
        let mut truncated_number = truncated_number.to_string();
        if truncated_number.len() > NUMBER_DECIMALS_SCIENTIFIC {
            Self::round(&mut truncated_number);
        }
        if let Some(mut digit) = truncated_number.pop() {
            while digit == '0' {
                digit = match truncated_number.pop() {
                    Some(x) => x,
                    None => break,
                }
            }
            truncated_number.push(digit);
        }
        // Only add decimal if we have more than one digit
        if truncated_number.len() > 1 {
            truncated_number.insert(1, '.'); // Decimal point
        }
        if length > NUMBER_DECIMALS_SCIENTIFIC + 1 {
            format!(
                "{}{} × 10^{}",
                if add_roughly { "roughly " } else { "" },
                truncated_number,
                length
            )
        } else {
            number.to_string()
        }
    }

    /// Rounds a base 10 number string.
    /// Uses the last digit to decide the rounding direction.
    /// Rounds over 9s. This does **not** keep the length or turn rounded over digits into zeros.
    /// If the input is all 9s, this will round to 10.
    ///
    /// # Panic
    /// This function may panic if less than two digits are supplied, or if it contains a non-digit of base 10.
    fn round(number: &mut String) {
        // Check additional digit if we need to round
        if let Some(digit) = number
            .pop()
            .map(|n| n.to_digit(10).expect("Not a base 10 number"))
        {
            if digit >= 5 {
                let mut last_digit = number
                    .pop()
                    .and_then(|n| n.to_digit(10))
                    .expect("Not a base 10 number");
                // Carry over at 9s
                while last_digit == 9 {
                    let Some(digit) = number
                        .pop()
                        .map(|n| n.to_digit(10).expect("Not a base 10 number"))
                    else {
                        // If we reached the end we get 10
                        *number = "10".to_string();
                        return;
                    };
                    last_digit = digit;
                }
                // Round up
                number.push_str(&format!("{}", last_digit + 1));
            }
        }
    }

    pub(crate) fn is_too_long(&self) -> bool {
        let n = match &self.factorial {
            CalculatedFactorial::Exact(n)
            | CalculatedFactorial::ApproximateDigits(n)
            | CalculatedFactorial::Approximate(_, n) => n,
        };
        n > &*TOO_BIG_NUMBER
    }

    pub(crate) fn get_factorial_level_string(level: i32) -> &'static str {
        let prefix = match level {
            -1 => "Sub",
            1 => "The ",
            2 => "Double-",
            3 => "Triple-",
            4 => "Quadruple-",
            5 => "Quintuple-",
            6 => "Sextuple-",
            7 => "Septuple-",
            8 => "Octuple-",
            9 => "Nonuple-",
            10 => "Decuple-",
            11 => "Undecuple-",
            12 => "Duodecuple-",
            13 => "Tredecuple-",
            14 => "Quattuordecuple-",
            15 => "Quindecuple-",
            16 => "Sexdecuple-",
            17 => "Septendecuple-",
            18 => "Octodecuple-",
            19 => "Novemdecuple-",
            20 => "Vigintuple-",
            21 => "Unvigintuple-",
            22 => "Duovigintuple-",
            23 => "Trevigintuple-",
            24 => "Quattuorvigintuple-",
            25 => "Quinvigintuple-",
            26 => "Sexvigintuple-",
            27 => "Septenvigintuple-",
            28 => "Octovigintuple-",
            29 => "Novemvigintuple-",
            30 => "Trigintuple-",
            31 => "Untrigintuple-",
            32 => "Duotrigintuple-",
            33 => "Tretrigintuple-",
            34 => "Quattuortrigintuple-",
            35 => "Quintrigintuple-",
            36 => "Sextrigintuple-",
            37 => "Septentrigintuple-",
            38 => "Octotrigintuple-",
            39 => "Novemtrigintuple-",
            40 => "Quadragintuple-",
            41 => "Unquadragintuple-",
            42 => "Duoquadragintuple-",
            43 => "Trequadragintuple-",
            44 => "Quattuorquadragintuple-",
            45 => "Quinquadragintuple-",
            _ => {
                let mut suffix = String::new();
                write!(&mut suffix, "{}-", level).unwrap();
                Box::leak(suffix.into_boxed_str())
            }
        };

        prefix
    }
}

impl Gamma {
    pub(crate) fn format(&self, acc: &mut String) -> Result<(), std::fmt::Error> {
        write!(
            acc,
            "{}{}{} is approximately {} \n\n",
            Factorial::get_factorial_level_string(1),
            PLACEHOLDER,
            self.number.as_float().to_f64(),
            self.gamma.as_float().to_f64()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::FLOAT_PRECISION;
    use rug::Integer;

    #[test]
    fn test_round_down() {
        let mut number = String::from("1929472373");
        Factorial::round(&mut number);
        assert_eq!(number, "192947237");
    }

    #[test]
    fn test_round_up() {
        let mut number = String::from("74836748625");
        Factorial::round(&mut number);
        assert_eq!(number, "7483674863");
    }

    #[test]
    fn test_round_carry() {
        let mut number = String::from("24999999995");
        Factorial::round(&mut number);
        assert_eq!(number, "25");
    }

    #[test]
    fn test_factorial_level_string() {
        assert_eq!(Factorial::get_factorial_level_string(1), "The ");
        assert_eq!(Factorial::get_factorial_level_string(2), "Double-");
        assert_eq!(Factorial::get_factorial_level_string(3), "Triple-");
        assert_eq!(
            Factorial::get_factorial_level_string(45),
            "Quinquadragintuple-"
        );
        assert_eq!(Factorial::get_factorial_level_string(50), "50-");
    }

    #[test]
    fn test_factorial_format() {
        let mut acc = String::new();
        let factorial = Factorial {
            number: 5.into(),
            levels: vec![1],
            factorial: CalculatedFactorial::Exact(Integer::from(120)),
        };
        factorial.format(&mut acc, false).unwrap();
        assert_eq!(acc, "The factorial of 5 is 120 \n\n");

        let mut acc = String::new();
        let factorial = Factorial {
            number: 5.into(),
            levels: vec![-1],
            factorial: CalculatedFactorial::Exact(Integer::from(120)),
        };
        factorial.format(&mut acc, false).unwrap();
        assert_eq!(acc, "Subfactorial of 5 is 120 \n\n");

        let mut acc = String::new();
        let factorial = Factorial {
            number: 5.into(),
            levels: vec![1],
            factorial: CalculatedFactorial::Approximate(
                Float::with_val(FLOAT_PRECISION, 1.2).into(),
                5.into(),
            ),
        };
        factorial.format(&mut acc, false).unwrap();
        assert_eq!(acc, "The factorial of 5 is approximately 1.2 × 10^5 \n\n");

        let mut acc = String::new();
        let factorial = Factorial {
            number: 5.into(),
            levels: vec![1],
            factorial: CalculatedFactorial::ApproximateDigits(3.into()),
        };
        factorial.format(&mut acc, false).unwrap();
        assert_eq!(acc, "The factorial of 5 has approximately 3 digits \n\n");

        let mut acc = String::new();
        let factorial = Factorial {
            number: 5.into(),
            levels: vec![1],
            factorial: CalculatedFactorial::Exact(Integer::from(120)),
        };
        factorial.format(&mut acc, true).unwrap();
        assert_eq!(acc, "The factorial of 5 is 120 \n\n");
    }
}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use super::*;

    // NOTE: The factorials here might be wrong, but we don't care, we are just testing the formatting

    #[test]
    fn test_format_factorial() {
        let fact = Calculation::Factorial(Factorial {
            number: 10.into(),
            levels: vec![3],
            factorial: CalculatedFactorial::Exact(280.into()),
        });
        let mut s = String::new();
        fact.format(&mut s, false).unwrap();
        assert_eq!(s, "Triple-factorial of 10 is 280 \n\n");
    }
    #[test]
    fn test_format_factorial_force_shorten_small() {
        let fact = Calculation::Factorial(Factorial {
            number: 10.into(),
            levels: vec![3],
            factorial: CalculatedFactorial::Exact(280.into()),
        });
        let mut s = String::new();
        fact.format(&mut s, true).unwrap();
        assert_eq!(s, "Triple-factorial of 10 is 280 \n\n");
    }
    #[test]
    fn test_format_factorial_force_shorten_large() {
        let fact = Calculation::Factorial(Factorial {
            number: 100.into(),
            levels: vec![1],
            factorial: CalculatedFactorial::Exact(
                Integer::from_str("232019615953125000000000000000000").unwrap(),
            ),
        });
        let mut s = String::new();
        fact.format(&mut s, false).unwrap();
        assert_eq!(
            s,
            "The factorial of 100 is 232019615953125000000000000000000 \n\n"
        );
    }
    #[test]
    fn test_format_factorial_auto_shorten() {
        let fact = Calculation::Factorial(Factorial {
            number: 3249.into(),
            levels: vec![1],
            factorial: CalculatedFactorial::Exact(
                Integer::from_str("64123376882765521838840963030568127691878727205333658692200854486404915724268122521695176119279253635876611090137291969570276913721864797759577004121543081865516901512445483449601769965060634861857064173938704305418376606356891014609023859758096597956259938348528946750437026172549655426092377089294607836520057856104816993984697675759579496157280331714452191401635250556082973306115574519424960196953201395066132365440977075392087489735146885581823595966673458107135041749084983583726462930633422893526599365244406644257808664472819062579590372326362859263197427382391737724371130194668325697913147795807287917882271125437793075279592752221056089408917956641344121781056494896664298954714463291743622978314854242079926982168325256172879601086193725507405771749789801611825741625380077209528888301112734777086106637653242107578812065387025070985682845983714635115865868052531038040737170581029905537322341939002838113744745962782070030988628668438192063964391415488312555937962867645737183703289987989371752808444472206166983181218698452231772212240017445423758860236449146575513014084114116542491422920779703202877962388772371297148878539228082497149672927873860981295756607109411429871735683677151117763870227460722732815888175758276344884954699572217509595160880510811349033936358665103889507929390456055037630508759624182491412136058522758117862715726418213812122827526330257260872329993280938592007320434494018056858434839424498517707440601396194949605570023576625190771463278168007414358018195714385208103590743168343592988436427551751120123934640886569178657972642734992568217335134536548423867468448461752994160896483162496996197629537563875663545967947035030506174219867102227347745166308776568259737417457622753953177779829173739659562549005900681020920836575654282170728038645671253311902327576757877160190593437037925134089334990083104974051379653937615220306281104735360028696101767109606466502484676624025302461421267416025443536877684785195571046059926349413586237838043863850610251583618438829618642246353724734656122845609571531588284708710081901687161770748138296656576032229319208279032435434327330035540657667361558905445221013396376775953367966087790302411507662731788873698999846238792500590360394500083923341408008981770566937535640769993694293230514231436990415482012055539596871513163008100690298424743718490882019179903258642028365049142613374709689558800856050749214398290563852574062566904927777093160819034619946818734041081848355062039645388238813669985569729968236449074797273410844560761607809842265309788155248298117938165414543689689754240992067831705834383207309250573018855640140957274364918049364842508738871690383100660359882462072065885517245667353800113210423157317762013988734352812105163694758108035856505778854524789188318600594132430921277654972526820920812190785994887939816114878915385423211996897729890266102145491069991647131611614465930571202528403443141981609375073983780241828798986101030035167624885608168623694530984934856402415662119456280967778213695343026782085453754332973412779641743296676142192492849866399186979810426206090031375249707803725234273693273721779240257093247268647749842459507965336971004339619911629224227060334233904444450352505466038312828689977755744971204784911189528493222070017894145493878499832441010771999957866634720057779638435426615168763950876432375766350648344132624416041623318009761058787995614968607413528076499437020919653085121078341947075546317831737787160036257151637941590867306372647047747729689844801136819011517526975033214302293538465503160183447374945622710595033673253137034231320031041035890947260824330728621640030383790059199531556893062561713763583025693789382680375603227866194301270004745201382665157844733507781537231595412109690534099208802055220457258238249940538761563309465648945964188442431661762589082015016756223358648046396366827537498425276338958018446839292802529780142385903309447658806351362744163752044896322012923382835852429065564336560491610071025646451525782856813152304143339115660276089535216189729579966851236899105440783686498435516601131545345163557980985342246336986737955743799192164259513473592703473521185371309681754246866522812455448210758136891890444056252857117200446002038652603259983493405505521897860879586618028713025173570291196046254005672495787117170419665767607647184551353826735583363126537373726390620854105626900247296291639985561481625404296348051054604042180512892657285238147263167051884385297470314430200590079012539964786079859359747123150407661818942489735756835032462952010303051169237940063644470670372188286551571968317499183600768353941744706305961785518398629201507525785967571188931895809109770264983907551256060144219899670118351808815620474425273993244741972143504134827047237929839845492209316520698259428270901257484509899386082594602760813392081897348940617781009158927227690469330327639146118508499255466535663882163793101115885899345523332216762566667486023534622719542192198250458735391090024294254053186440646305309340840685145289223131431157156390489399333752075193525158125680201419183806547205312873264380358849214095835479613319512867197427682723250079990981586869733293245764804577570764831692705888317075918673294669326798053736223321604803330275717540789920865913177228227111643923604665959921096208765542277777829882980225810940866410254096689483571105776785837917708633884075471298045453873223073787369262426626913405098535070631297346400765749139515252242178612533747493270131589184346851060077512732273563896936880596142362061341020737937605198462006142952423931616201569440226926787162077801883794168906567939864710313203688516686488132607069944238278930371283198545637735863991249832218463680910774912311493673518088306563853170521159963238305666024221618323515872866318153226269712890565361382209276094137857215708859605439920538254391240145615109307534437972388439697355227469268959991826344643967606862639207957142695059497774782782862380576527665249011786632721781635858363134217267161265609789721847126531549373639397319541419174824349828634414533913160986280670700117904134971824878639490677063427559640621162799757094469987184056964512589036737188936656494184932005003301076625555129466247988108160104882718140259576746243025950653945267030862681712132414998384138315991964228278130346276982182371619123375659027762342810200791337975076096607162500887202849331840711439619934443487228446573730294798389422723901661778354768525095757656920903185278358954945675520361768231577076750321654682566951617894418024879897723932943778739392625374786945631297844013055183788373235917906391604745846654356151085578611880261515860397623972021392725059655970516681719822949498069366408864396412928494605832710960284204937215373010567096882590065428759248976242854170628853902061231484918006271406155707387649451852150396381227895427254475130432845540997751264574249884576973754475522081887586009543117655192564603663203594121977491966995919938707026254622729082886656923266824175261927609862131917883084745112234024557978747561458733390353402381353061864973111801478933098174668694254024372053350135966105816774315863351432700501507214833910835095241116220945368287364828423032249431110250529198415073098056537298790818802403747860478015395740166511031245261193793854201285682331906071528112005073514650997116494101706639070013374677115821301361236988511929513457351929738018793684759539098410509535113338894579685309152120362751957602730649344150813012563246391457667149097699631546631367291707994927436193366185835774355812730356484690902974319470019544218388669048171395399380611906621586431005917959473642252829970939300283923684023821586277795276767391621510747281802893209607052311085173753725616353413592446675522238914835135290803927878090361225614843018882327106532840756094139114333346621153175254833577042328095480536834801026590432360931424294133543336408702705440236553526213058195627059654976746315636170233701887454392139871178240463495036735780991998499617099173145932919728906603992606395026374552882029156921168342421270810263586384930758466962518032019544198713384832174173447126633137813741748004660781750992387224960402183367639878315847417040125065349322346833085734948541674565230896990919815801676540094611430605654337096768783494147476599630304276589463660992695730097812987784061106253993478908686689107637583574009574525664941872851644555317421340687668414081763994364249671165252652825318436095248164540239487724330276498957490699548343852181838068378612444949106850962864407345130509165857647406496109100001533123176834579856292423765079015513705518869769002090306548513909235083737585930276738943593954668225536658208962591163051195501324651032924378645456520478535714079874404144783894706654731307268880764144813567558473827034967105368425271973138213726718055181321006250745589786136935583735915890517993411416086214277469794370188740010736604373520529352427775875772577651690552630708696044935360500197728514057299685757816479040563926362665221456966339198099627395349937057349473111399655105587183432516687910987518148931239145857422059143761070545360054386871218955184209375241453611589548642653321253873363792347807426924575722280463634222994099258528815002881358362491008896204800000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap()
            ),
        });
        let mut s = String::new();
        fact.format(&mut s, false).unwrap();
        assert_eq!(
            s,
            "The factorial of 3249 is roughly 6.412337688276552183884096303057 × 10^10000 \n\n"
        );
    }
    #[test]
    fn test_format_factorial_chain() {
        let fact = Calculation::Factorial(Factorial {
            number: 5.into(),
            levels: vec![3, 1],
            factorial: CalculatedFactorial::Exact(3628800.into()),
        });
        let mut s = String::new();
        fact.format(&mut s, false).unwrap();
        assert_eq!(s, "The factorial of Triple-factorial of 5 is 3628800 \n\n");
    }
    #[test]
    fn test_format_approximate_factorial() {
        let fact = Calculation::Factorial(Factorial {
            number: 0.into(),
            levels: vec![1],
            factorial: CalculatedFactorial::Approximate(
                Float::with_val(FLOAT_PRECISION, 2.83947).into(),
                10043.into(),
            ),
        });
        let mut s = String::new();
        fact.format(&mut s, false).unwrap();
        assert_eq!(
            s,
            "The factorial of 0 is approximately 2.83947 × 10^10043 \n\n"
        );
    }
    #[test]
    fn test_format_approximate_digits_factorial() {
        let fact = Calculation::Factorial(Factorial {
            number: 0.into(),
            levels: vec![1],
            factorial: CalculatedFactorial::ApproximateDigits(10043394.into()),
        });
        let mut s = String::new();
        fact.format(&mut s, false).unwrap();
        assert_eq!(
            s,
            "The factorial of 0 has approximately 10043394 digits \n\n"
        );
    }
    #[test]
    fn test_format_gamma() {
        let fact = Calculation::Gamma(Gamma {
            number: Float::with_val(FLOAT_PRECISION, 9.2).into(),
            gamma: Float::with_val(FLOAT_PRECISION, 893.83924421).into(),
        });
        let mut s = String::new();
        fact.format(&mut s, false).unwrap();
        assert_eq!(s, "The factorial of 9.2 is approximately 893.83924421 \n\n");
    }
    #[test]
    fn test_format_approximate_factorial_shorten() {
        let fact = Calculation::Factorial(Factorial {
            number: Integer::from_str("2018338437429423744923849374833232131").unwrap(),
            levels: vec![1],
            factorial: CalculatedFactorial::Approximate(
                Float::with_val(FLOAT_PRECISION, 2.8394792834).into(),
                Integer::from_str("10094283492304894983443984102489842984271").unwrap(),
            ),
        });
        let mut s = String::new();
        fact.format(&mut s, true).unwrap();
        assert_eq!(
            s,
            "The factorial of 2.018338437429423744923849374833 × 10^36 is approximately 2.8394792834 × 10^(1.009428349230489498344398410249 × 10^40) \n\n"
        );
    }
    #[test]
    fn test_format_approximate_digits_factorial_shorten() {
        let fact = Calculation::Factorial(Factorial {
            number: Integer::from_str("2313820948092579283573259490834298719").unwrap(),
            levels: vec![1],
            factorial: CalculatedFactorial::ApproximateDigits(
                Integer::from_str("9842371208573508275237815084709374240128347012847").unwrap(),
            ),
        });
        let mut s = String::new();
        fact.format(&mut s, true).unwrap();
        assert_eq!(
            s,
            "The factorial of 2.313820948092579283573259490834 × 10^36 has approximately 9.842371208573508275237815084709 × 10^48 digits \n\n"
        );
    }
    #[test]
    fn test_format_huge() {
        let fact = Calculation::Factorial(Factorial {
            number: 0.into(),
            levels: vec![1],
            factorial: CalculatedFactorial::Exact({
                let mut r = Float::with_val(FLOAT_PRECISION, rug::float::Special::Infinity);
                r.next_down();
                r.to_integer().unwrap()
            }),
        });
        let mut s = String::new();
        fact.format(&mut s, false).unwrap();
        assert_eq!(
            s,
            "The factorial of 0 is roughly 2.098578716467387692404358116884 × 10^323228496 \n\n"
        );
    }
}
