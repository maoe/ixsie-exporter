use std::{
    cmp::PartialOrd,
    fmt::Display,
    io::{self, Write},
    str::FromStr,
    sync::Arc,
};

use anyhow::{anyhow, bail};
use chrono::Month;
use futures::stream::StreamExt;
use num_traits::FromPrimitive;
use regex::Regex;
use structopt::StructOpt;
use tokio::io::AsyncWriteExt;

#[derive(StructOpt, Debug)]
struct Opt {
    /// Starting month of the export
    #[structopt(long, value_name = "YEAR-MONTH", default_value = "2018-03")]
    from: YearMonth,
    /// Ending month of the export
    #[structopt(long, value_name = "YEAR-MONTH", default_value = "2023-03")]
    to: YearMonth,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct YearMonth {
    year: i32,
    month: Month,
}

impl PartialOrd for YearMonth {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        let same_year = self.year.partial_cmp(&other.year)?;
        let same_month = self
            .month
            .number_from_month()
            .partial_cmp(&other.month.number_from_month())?;
        Some(same_year.then(same_month))
    }
}

impl Iterator for YearMonth {
    type Item = Self;

    fn next(&mut self) -> Option<Self::Item> {
        let this = *self;
        self.month = this.month.succ();
        if self.month.number_from_month() < this.month.number_from_month() {
            self.year += 1;
        }
        Some(this)
    }
}

impl Display for YearMonth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}-{:02}", self.year, self.month.number_from_month())
    }
}

impl FromStr for YearMonth {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r"(\d{4})-(\d{2})")?;
        let cap = re
            .captures(s)
            .ok_or_else(|| anyhow!("YYYY-MM形式で入力してください: {}", s))?;
        Ok(YearMonth {
            year: cap[1].parse()?,
            month: Month::from_u64(cap[2].parse::<u64>()?)
                .ok_or_else(|| anyhow!("月は1-12の間の数字にしてください: {}", &cap[2]))?,
        })
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let opt = Opt::from_args();
    let mut login_address = String::new();
    print!("ログインメールアドレス: ");
    io::stdout().flush()?;
    io::stdin().read_line(&mut login_address)?;
    let login_password = rpassword::prompt_password("ログインパスワード: ")?;
    let client = Arc::new(reqwest::Client::builder().cookie_store(true).build()?);
    let form = reqwest::multipart::Form::new()
        .text("loginId", login_address)
        .text("loginPass", login_password);
    let body = client
        .post("https://app.ixsie.jp/signin")
        .multipart(form)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    if !body.contains("ログアウト") {
        bail!("Failed to log in");
    }
    let urls = opt.from.take_while(|month| *month <= opt.to).map(|month| {
        let url = format!(
            "https://app.ixsie.jp/user/contact/pdf?contactYear={}&contactMonth={}",
            month.year,
            month.month.number_from_month()
        );
        (month, url)
    });
    let mut stream = tokio_stream::iter(urls)
        .map(|(month, url)| {
            let client = Arc::clone(&client);
            async move {
                let resp = client.get(url).send().await;
                (month, resp)
            }
        })
        .buffer_unordered(4);
    while let Some((month, resp)) = stream.next().await {
        let mut resp = resp?.error_for_status()?;
        println!("{month}");
        let mut outfile = tokio::fs::File::create(format!("{month}.pdf")).await?;
        while let Some(chunk) = resp.chunk().await? {
            outfile.write_all(&chunk).await?;
        }
        outfile.flush().await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_year_month() {
        assert_eq!(
            YearMonth::from_str("2020-01").unwrap(),
            YearMonth {
                year: 2020,
                month: Month::January
            }
        )
    }

    #[test]
    fn invalid_year_month() {
        assert!(YearMonth::from_str("2020-13").is_err())
    }

    #[test]
    fn year_month_partial_cmp() {
        let dec_2020 = YearMonth {
            year: 2020,
            month: Month::December,
        };
        let jan_2021 = YearMonth {
            year: 2021,
            month: Month::January,
        };
        let feb_2021 = YearMonth {
            year: 2021,
            month: Month::February,
        };
        assert!(dec_2020 < jan_2021);
        assert!(jan_2021 < feb_2021);
    }

    #[test]
    fn year_month_iter() {
        let dec_2020 = YearMonth {
            year: 2020,
            month: Month::December,
        };
        assert_eq!(
            dec_2020.take(3).collect::<Vec<_>>(),
            vec![
                YearMonth {
                    year: 2020,
                    month: Month::December
                },
                YearMonth {
                    year: 2021,
                    month: Month::January
                },
                YearMonth {
                    year: 2021,
                    month: Month::February
                }
            ]
        )
    }
}
