use reqwest;
use serde_json::Value;
use ta::indicators::RelativeStrengthIndex;
use ta::{Next, DataItem};

const BYBIT_TICKERS_API: &str = "https://api.bybit.com/v5/market/tickers";
const BYBIT_KLINE_API: &str = "https://api.bybit.com/v5/market/kline";
const CATEGORY: &str = "linear"; // Mercado perpétuo USDT
const INTERVAL: &str = "240"; // 4 horas
const RSI_PERIOD: usize = 14;
const LIMIT: usize = 100; // Número máximo de moedas a buscar

#[tokio::main]
async fn main() {
    match fetch_top_futures_usdt().await {
        Ok(symbols) => {
            println!("🔍 Verificando RSI para as {} melhores moedas de futuros USDT com valores menores que 30 e maiores que 70...\n", symbols.len());

            let mut rsi_below_30 = Vec::new();
            let mut rsi_above_70 = Vec::new();

            for symbol in &symbols {
                match fetch_rsi(symbol).await {
                    Ok(Some(rsi)) if rsi < 30.0 => rsi_below_30.push((symbol.clone(), rsi)),
                    Ok(Some(rsi)) if rsi > 70.0 => rsi_above_70.push((symbol.clone(), rsi)),
                    Ok(_) => (),
                    Err(e) => eprintln!("⚠️ Erro ao buscar RSI para {}: {}", symbol, e),
                }
            }

            println!("📉 Moedas com RSI abaixo de 30 (Sobrevenda):");
            if rsi_below_30.is_empty() {
                println!("Nenhuma moeda está com RSI abaixo de 30 no momento.");
            } else {
                for (symbol, rsi) in &rsi_below_30 {
                    println!("➡️ {} | RSI: {:.2}", symbol, rsi);
                }
            }

            println!("\n📈 Moedas com RSI acima de 70 (Sobrecompra):");
            if rsi_above_70.is_empty() {
                println!("Nenhuma moeda está com RSI acima de 70 no momento.");
            } else {
                for (symbol, rsi) in &rsi_above_70 {
                    println!("➡️ {} | RSI: {:.2}", symbol, rsi);
                }
            }
        }
        Err(e) => eprintln!("❌ Erro ao buscar moedas: {}", e),
    }
}

// Busca as 100 principais moedas de futuros USDT
async fn fetch_top_futures_usdt() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let url = format!("{}?category={}", BYBIT_TICKERS_API, CATEGORY);
    let response = reqwest::get(&url).await?.text().await?;
    let json: Value = serde_json::from_str(&response)?;

    if json["result"]["list"].is_null() || !json["result"]["list"].is_array() {
        return Err("Resposta da API não contém os dados esperados".into());
    }

    let tickers = json["result"]["list"]
        .as_array()
        .ok_or("Erro ao obter lista de moedas")?;

    let symbols: Vec<String> = tickers.iter()
        .filter_map(|ticker| ticker["symbol"].as_str().map(String::from))
        .take(LIMIT)
        .collect();

    Ok(symbols)
}

// Busca o RSI de uma moeda específica
async fn fetch_rsi(symbol: &str) -> Result<Option<f64>, Box<dyn std::error::Error>> {
    let url = format!(
        "{}?category={}&symbol={}&interval={}&limit=100",
        BYBIT_KLINE_API, CATEGORY, symbol, INTERVAL
    );

    let response = reqwest::get(&url).await?.text().await?;
    let json: Value = serde_json::from_str(&response)?;

    if json["result"]["list"].is_null() || !json["result"]["list"].is_array() {
        return Err(format!("Nenhuma vela disponível para {}", symbol).into());
    }

    let candles = json["result"]["list"]
        .as_array()
        .ok_or("Erro ao obter velas")?;

    let mut closes: Vec<f64> = Vec::new();
    for candle in candles {
        if let Some(close_str) = candle[4].as_str() { // O preço de fechamento está na posição 4
            if let Ok(close) = close_str.parse::<f64>() {
                closes.push(close);
            }
        }
    }

    if closes.len() < RSI_PERIOD {
        return Ok(None);
    }

    let mut rsi = RelativeStrengthIndex::new(RSI_PERIOD).unwrap();
    for &close in &closes {
        let item = DataItem::builder()
            .open(close)
            .high(close)
            .low(close)
            .close(close)
            .volume(1.0) // Definir um volume fictício
            .build()
            .unwrap();
        rsi.next(&item);
    }

    let last_item = DataItem::builder()
        .open(*closes.last().unwrap())
        .high(*closes.last().unwrap())
        .low(*closes.last().unwrap())
        .close(*closes.last().unwrap())
        .volume(1.0)
        .build()
        .unwrap();

    Ok(Some(rsi.next(&last_item)))
}
