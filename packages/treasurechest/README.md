# Treasure chest Packages

This is a variant of a vault.

1. ownership/admin of a tokenfactory token (ticket token) is sent to the contract
2. A pool of funds (no CW20s). We calculate the value of the pool per 1 token / outstanding tickets (of each of the reward tokens)

2. People would call the claim() function and would send the X ticket tokens with claim. This would then multiply X * value_of_one_token
3. It would burn the tokens received, and send the tokens to the caller.


## limitations

* pool of reward tokens will be limited to N tokens ( where N < 20) so that 
Bank::Send will be able to send them all in one transaction

* rounding dust errors will occur.