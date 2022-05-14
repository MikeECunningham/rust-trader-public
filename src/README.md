## General Info
-The trader maintains two primary threads: market modelling (src/signal_handler), and account modelling/strategy (src/strategy).  
-The market thread pipeline is to receive market updates from REST/websocket threads, update respective models, generate an analysis result, and push it to the strategy thread.  
-Order book and trade flow models are kept in the src/orderbook and src/tradeflow folders. Analysis can be found in src/analysis.  
-The strategy thread pipeline is to update account models with account and order updates, and execute orders with market model updates.  
-REST and websocket connectors are kept in src/backend.  

This project makes use of the dec library  
 https://docs.rs/dec/latest/dec/#  
 It is provided in src/dec with modifications under its original license to solve a number of internal problems with using it as a crate.
