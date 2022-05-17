The strategy folder contains account modelling and the runner for order execution: strategy.rs.  
Strategies are event listeners for account and market information.

## Account Model
The account model is divided by concern:
1. Account will hold strategies for each symbol, which is TODO.
2. Strategies hold a portfolio that they act on, representing all broker actions to be taken on a market for a symbol.
3. Portfolios are always in hedge mode, and internally relate the long position to the short position.
4. Positions relate lists of "opening" (inventory accumulating) orders to lists of "closing" (inventory reducingg orders).
5. Order lists hold, maintain, relate and examine orders at any state in an order's lifespan.
6. Orders represent individual orders. Orders have life stages, from "in flight" to resting, partially filled, filled, or failed/cancelled in some way. They also have internal states which relate to the demo strategy. Orders can be limit or market, it's the same type for each.

The account model also performs a self-analysis step, through its data_refresh() function, to populate the PortfolioData property. This is greedier than simply updating the relevant information as changes are made, but simpler to implement. A proper implementation is TODO.  
This data conveys cumulative information about the current portfolio state.

## Strategy
The strategy event listener matches a message enum by type, then passes the message to its handler function. In the current demo, account messages update the model but don't generally warrant a response. Orders are placed based on market information.  
At the time of writing, Binance offers a flat maker rebate on BUSD perpetuals. The demo strategy simply attempts to push through as many maker orders as possible.  
Tops: strategy seeks to keep top level orders at all time, using Binance's real-time best levels websocket.  
Orderbook: strategy seeks to populate lower orders at specific intervals based on the 250ms order book websocket, with the aim of mitigating adverse sweeps. These orders' are placed with the lowest quantity, and furthest price, that will allow the cost basis to be reset (through the maker rebate) to that order's limit price, without producing any "gaps" in which a fill would not cover the amount lost by the sweep.

It cannot be emphasized enough that the demo is very "quick and dirty", and on top of a heavily WIP system. The math occasionally does not check out (leading to losing closes), inventory rapidly accumulates during aggressive sweeps, and the pnl report on a closed position is inaccurate. Fast response times should be rewarded, but a sanity check on the Tokyo AWS server at <10ms round-trip revealed an apparent fatal glitch in the strategy logic, causing it to lock up from repeated attempts to cancel an unknown order.