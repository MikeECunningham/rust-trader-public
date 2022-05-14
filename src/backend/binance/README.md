Backend manages the connection to Binance.  
/broker manages order execution REST endpoints, /market manages market REST endpoints, and /stream manages all websockets.  
Both REST managers are lazy-static objects that are started in main and made available to their corresponding threads. The broker object keeps a timestamp that it attempts to keep synchronized with the Binance server. It is initiated through the time request and can be shifted by digesting incoming timestamp errors.

Types are stored mainly in backend/[exchange]/types, with some universal types kept in backend/types. This project uses serde to ingest, format, and prepare data as concisely as possible.
