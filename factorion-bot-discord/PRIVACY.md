# Privacy Policy
## Working Data
To be able to find and answer to factorials, the bot reads all messages in the channels it is active in.

The message content is not stored if the server is configured correctly.
This is due to the discord client framework, we use, which logs all Events.
The official bot `factorion-bot` with the id `1425936019559153847`, is configured to supress these logs.

## Stored Data
Channel configuration, which inludes pre-set commands and locale, is saved.
Additionally for development, debugging and statistical purposes, some information is logged and permanently saved.

This information is saved, when factorion finds operations (factorials or similar):
  - comment author name
  - message and channel id
  - the calculations which include:
    - the parsed numbers and operations
    - the calculated result
  - some status information which includes:
    - whether factorion replied
    - whether some operation could not be calculated
  - which commands were applied (set by user or configured for channel)
  - which locale was used

This information may be saved on errors:
  - message and channel id
  - information from comment excluding comment text.

This information may be saved for any comment:
  - time the message was recieved
  - time taken to parse, calculate and format individually

## Shared Data
Some statistics may be shared with the public.

Such statistics may include:
  - time taken for parsing, calculation and formatting
  - channel ids with number and time of factorials
  - author names with number of factorials
  - calculation statistics wich may include (anonymously) all information regarding individual calculations as defined above
  - (anonymous) command statistics
