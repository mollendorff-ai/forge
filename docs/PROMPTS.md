# Prompts for your AI

Forge is made to be used by an AI: it writes the model as YAML, Forge does the arithmetic, and every number traces back to a formula. Each prompt below can be pasted as it is into an AI that can run commands. Each one:
- installs Forge;
- does one kind of work a spreadsheet user does every day;
- asks the AI to check one result its own way, rather than repeat Forge's numbers;
- ends with the model's YAML, which you can open in [Forge Studio](https://forge.mollendorff.ai/).

**Where they work:** the AI has to be able to run commands. Tried by the maintainer on 2026-11-15, the first prompt (in the [README](../README.md#try-it-with-your-ai)) worked on Claude and on Grok, free tiers included. The free tiers of ChatGPT and Gemini had no sandbox to run it in. Each prompt asks an AI that cannot run it to say so, not to make numbers up.

**Each prompt here was run by a fresh agent** (one that had never seen Forge) before it went in. The date and the outcome are under each prompt.

**Opening the result in Studio:**
- **A model in one file:** open Studio, go to the YAML tab, select all, paste the AI's YAML over it, and press "Check and apply".
- **A model in several files** (one that includes another): save each file with its name, then open them together from Studio's Files tab, with "Open YAML files from this device".

## 1. Build a model from a description

The everyday case: a plan described in words, turned into a model you can read, change and send.

```text
Use Forge, the financial-modelling engine from Möllendorff AI
(https://github.com/mollendorff-ai/forge; the crate is mollendorff-forge).
Do not build it from source; install it with:

  curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh -s -- --examples

Build a 12-month plan for a small coffee shop as a Forge model. A cup sells
for $4.50; it sells 180 cups a day in month 1, growing 2% a month; a month
is 30 days; coffee and milk cost $1.10 a cup; rent is $3,200 a month; two
staff work 10 hours a day each at $18 an hour.

Put the assumptions in named scalars, a 12-row months table with revenue,
costs and profit, and the year's totals. Run `forge examples starter` first
to see the format, and `forge validate` before `forge calculate`. After you
change a formula, run `forge calculate` again: until then `forge validate`
says the stored values are out of date.

Then tell me the year's revenue and profit and the worst month, and show
the formula behind the profit. Check the year's revenue yourself with your
own code and say whether it agrees.

At the end, print every YAML file you wrote, in full, each in its own code
block with its file name, so I can open it in Forge Studio.

After this I will keep asking you questions about this model and what-ifs;
answer from the model and Forge itself, and use `forge --help` and
`forge <command> --help` for anything else Forge can do.

If you cannot run commands here, say so plainly and do not invent any
results. Point me to Forge Studio instead (https://forge.mollendorff.ai/).
```

*Run on 2026-11-22 by a fresh agent (Forge 10.0.0-rc.33): installed in 2.8 s; a coffee-shop model with assumptions, a 12-month table and year totals, `forge validate` passing before and after `forge calculate`; year revenue 325,913.78 and profit 78,245.97, worst month the first (4,360); its own check, month by month in Python and in closed form, agreed to the last digit. Under a minute in all.*

## 2. Bring a workbook in from Excel, and back

Forge reads a workbook, keeps the formulas it can express, and writes a workbook with live formulas.

```text
Use Forge, the financial-modelling engine from Möllendorff AI
(https://github.com/mollendorff-ai/forge; the crate is mollendorff-forge).
Do not build it from source; install it with:

  curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh -s -- --examples

Work on an Excel workbook. If I attached one, use it. If not, make one from
the bagel example: `forge export forge-examples/bagel/bagel.yaml bagel.xlsx`.

1. Import it with `forge import` and tell me what Forge kept as formulas and
   what it brought in as values (it says why).
2. Calculate the imported model and compare a few results with the values
   the workbook had stored, which you can read with your own code.
3. Change one input in the YAML, calculate again, and export a new workbook
   with `forge export`. Tell me what changed.

At the end, print every YAML file you wrote, in full, each in its own code
block with its file name, so I can open it in Forge Studio.

After this I will keep asking you questions about this model and what-ifs;
answer from the model and Forge itself, and use `forge --help` and
`forge <command> --help` for anything else Forge can do.

If you cannot run commands here, say so plainly and do not invent any
results. Point me to Forge Studio instead (https://forge.mollendorff.ai/).
```

*Run on 2026-11-22 by a fresh agent (Forge 10.0.0-rc.33), with no workbook attached: it exported the bagel example and imported it back (all 18 formulas kept, the inputs as values, `costs.yaml` as its own file), and found the results equal to the workbook's stored values to 15 significant digits, read with its own openpyxl script. The price changed to 17 gave a profit of 24,334.44 and a new workbook. It noted that scenarios, the tornado and comments do not go through Excel, as the export says. About 80 s.*

## 3. Budget against actual

A month's plan and what happened, line by line, with which way each line is good news.

```text
Use Forge, the financial-modelling engine from Möllendorff AI
(https://github.com/mollendorff-ai/forge; the crate is mollendorff-forge).
Do not build it from source; install it with:

  curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh -s -- --examples

1. Run `forge variance forge-examples/household/budget.yaml
   forge-examples/household/actual.yaml --threshold 10` and tell me in plain
   words where the month went off plan, and which differences are good news.
   Check two of the variances yourself with your own code.
2. Write the same pair of files for my own month and compare them the same
   way. Planned / actual: salary 4,800 / 4,800; rent 1,450 / 1,450;
   groceries 600 / 742; transport 180 / 95; eating out 200 / 310.

At the end, print every YAML file you wrote, in full, each in its own code
block with its file name, so I can open it in Forge Studio.

After this I will keep asking you questions about this model and what-ifs;
answer from the model and Forge itself, and use `forge --help` and
`forge <command> --help` for anything else Forge can do.

If you cannot run commands here, say so plainly and do not invent any
results. Point me to Forge Studio instead (https://forge.mollendorff.ai/).
```

*Run on 2026-11-22 by a fresh agent (Forge 10.0.0-rc.33): installed in 2.7 s; the household month 455.85 short of plan, side work and eating out the main causes and transport the only good news; two variances checked in its own Python script; the user's month 167 short, food the whole story. About 50 s.*

## 4. What if: scenarios, break-even, goal seek

The questions asked of every plan: what if things go differently, where is the line, what would it take.

```text
Use Forge, the financial-modelling engine from Möllendorff AI
(https://github.com/mollendorff-ai/forge; the crate is mollendorff-forge).
Do not build it from source; install it with:

  curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh -s -- --examples

Use the bagel shop's plan, forge-examples/bagel/bagel.yaml (it reads
costs.yaml beside it). Use `forge <command> --help` when you need the options.

1. `forge scenarios`: what each scenario does to the year's profit, and the
   probability-weighted result.
2. `forge break-even`: the price per dozen at which the year's profit is zero.
3. `forge goal-seek`: the staff hours a day for a profit of 60,000.
4. `forge sensitivity`: the year's profit for prices from 14 to 20 a dozen.

Then tell the owner, in plain words, what to take from it. Check the
break-even price yourself: put it in a copy of the model, calculate, and
show that the profit is about zero.

At the end, print every YAML file you wrote or changed, in full, each in its
own code block with its file name, so I can open it in Forge Studio.

After this I will keep asking you questions about this model and what-ifs;
answer from the model and Forge itself, and use `forge --help` and
`forge <command> --help` for anything else Forge can do.

If you cannot run commands here, say so plainly and do not invent any
results. Point me to Forge Studio instead (https://forge.mollendorff.ai/).
```

*Run on 2026-11-22 by a fresh agent (Forge 10.0.0-rc.33): scenarios (probability-weighted profit 621.64), break-even at 15.97 a dozen, 24.27 staff hours a day for 60,000, profit from -46,310 at 14 to 94,979 at 20; with the break-even price worked out to 15.9666, a copy of the model made -0.19. `--vary price_per_dozen` now works without the group name. About 75 s.*

## 5. Risk and decisions

When the inputs are uncertain, and when the question is which way to go.

```text
Use Forge, the financial-modelling engine from Möllendorff AI
(https://github.com/mollendorff-ai/forge; the crate is mollendorff-forge).
Do not build it from source; install it with:

  curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh -s -- --examples

The bagel shop again, in forge-examples/bagel/. Use `forge <command> --help`
when you need the options.

1. `forge simulate bagel_simulate.yaml`: how likely the first year is to make
   money, what the worst years look like, and what drives the result most.
2. `forge tornado bagel.yaml`: which inputs swing the year's profit most.
3. `forge decision-tree second_shop.yaml`: open a second shop now, run a
   pop-up stall first, or stay with one shop?
4. `forge real-options wait_option.yaml`: what is the right to wait a year
   worth?

Explain each in plain words for the owner. Check the decision tree yourself:
work out the expected value of each choice by hand from second_shop.yaml.

At the end, print every YAML file you wrote or changed, in full, each in its
own code block with its file name, so I can open it in Forge Studio. If you
changed none, say so.

After this I will keep asking you questions about this model and what-ifs;
answer from the model and Forge itself, and use `forge --help` and
`forge <command> --help` for anything else Forge can do.

If you cannot run commands here, say so plainly and do not invent any
results. Point me to Forge Studio instead (https://forge.mollendorff.ai/).
```

*Run on 2026-11-22 by a fresh agent (Forge 10.0.0-rc.33): a 35% chance of a profitable first year over 5,000 runs, opening demand the main driver (+0.84); the tornado led by opening demand, price and staff hours; the decision tree's 58,500 (open now) and 61,500 (pop-up first) worked out again by hand; waiting a year worth 4,605 more than investing today. No file changed. About 60 s.*

## 6. Games: what will the other side do?

When the answer depends on what someone else chooses: an AI that may cut corners and a human who may check, a rival who may cut prices, partners who may or may not cooperate.

```text
Use Forge, the financial-modelling engine from Möllendorff AI
(https://github.com/mollendorff-ai/forge; the crate is mollendorff-forge).
Do not build it from source; install it with:

  curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh -s -- --examples

Two games in forge-examples/games/; run the commands from forge-examples/.
Use `forge game --help` when you need the options.

1. `forge game games/inspection.yaml`: an AI complies or cheats, a human
   audits or trusts. Say who does what, and how often, in the equilibrium.
   Check it yourself: show by hand that each side is indifferent between its
   two choices when the other plays that mix.
2. Copy inspection.yaml, raise `penalty` from 4 to 8 in the copy, and run
   `forge game` on it. What happens to the audit rate, and to the cheating?
   Explain the result in plain words; it surprises people.
3. `forge game games/axelrod.yaml`: a prisoner's dilemma, then a tournament
   of repeated play between five strategies. Who wins, who is "collectively
   stable", and why does the dilemma's single-round answer differ from the
   tournament's? Check one cell of the match table by hand (for example
   tit for tat against always defect over 200 rounds).

At the end, print every YAML file you wrote or changed, in full, each in its
own code block with its file name, so I can open it in Forge Studio. If you
changed none, say so.

After this I will keep asking you questions about these games and what-ifs;
answer from the files and Forge itself, and use `forge --help` and
`forge <command> --help` for anything else Forge can do.

If you cannot run commands here, say so plainly and do not invent any
results. Point me to Forge Studio instead (https://forge.mollendorff.ai/).
```

*Run on 2026-11-22 by a fresh agent (Forge 10.0.0-rc.33): the inspection game's one mixed equilibrium (the AI cheats a third of the time, the human audits a third of the time), each side's indifference shown by hand; with the penalty at 8 the audit rate falls to a fifth and the cheating stays at a third, explained as each side's mix being set by the other side's payoffs; in the tournament grim trigger first at 2.596 a round, tit for tat second at 2.448, grim, tit for tat and always defect collectively stable; tit for tat against always defect worked out by hand as 0.995 (and always defect's 1.020), as the match table says. It wrote one copy, `inspection-penalty8.yaml`. No command failed; about 16 s of commands. The only question it had was the folder: it ran from `forge-examples/`, which the prompt now says.*

## 7. A price war: games from the model's own numbers

The games of prompt 6, built from a real plan: the bagel shop's margins become the payoffs, played once, month after month, and across a city of shops.

```text
Use Forge, the financial-modelling engine from Möllendorff AI
(https://github.com/mollendorff-ai/forge; the crate is mollendorff-forge).
Do not build it from source; install it with:

  curl -fsSL https://raw.githubusercontent.com/mollendorff-ai/forge/main/install.sh | sh -s -- --examples

The bagel shop again: forge-examples/bagel/price_war.yaml, a price war with
the shop across the street. Run the commands from forge-examples/. Use
`forge game --help` and `forge <command> --help` when you need the options.

1. `forge calculate bagel/price_war.yaml --dry-run`, then
   `forge game bagel/price_war.yaml`: played once, should Bagel Lune hold
   $16 a dozen or cut to $14, and what does that cost each shop a month?
   Show which formulas the four payoffs come from.
2. In the same output, the tournament (`rivalry`): month after month, who
   earns most, who is "collectively stable", and why are those not the same
   strategies? Check one cell yourself, by hand: grim trigger against the
   discounter, with a 0.95 chance each month that both shops are still open.
3. The dynamics: if a tenth of the city's bagel shops cut today and owners
   copy whoever earns more, where is the city in two years?
4. Run it again with `--scenario loyal_regulars` and `--scenario lease_ending`.
   What changes, and what would you tell the owner to do in each case?

Explain everything in plain words for the owner.

At the end, print every YAML file you wrote or changed, in full, each in its
own code block with its file name, so I can open it in Forge Studio. If you
changed none, say so.

After this I will keep asking you questions about this model and what-ifs;
answer from the model and Forge itself, and use `forge --help` and
`forge <command> --help` for anything else Forge can do.

If you cannot run commands here, say so plainly and do not invent any
results. Point me to Forge Studio instead (https://forge.mollendorff.ai/).
```

*Run on 2026-11-22 by a fresh agent (Forge 10.0.0-rc.33): played once, cutting dominates and both shops lose 2,300 a month against both holding, with the four payoffs traced to their formulas (food 2.56 a dozen, margins 13.02 and 11.08); in the tournament the discounter earns most (21,065) by feeding on the shops that always hold or forgive, while tit for tat, grim trigger and the discounter are collectively stable, the threshold 0.640; grim trigger against the discounter worked out by hand as 0.05 x 12,893.63 + 0.95 x 19,188.94 = 18,874.17, as Forge's cell; across the city 99.1% of the shops cut in two years; with loyal regulars holding dominates and the cutters fall to 2.4%; with the lease ending (0.6) only the tournament changes and the discounter is the only stable strategy. No file changed, no command failed, under 5 s of commands. It noticed that the example's comment said grim trigger "cannot be beaten by a discounter" when it is beaten head to head; the comment now says collectively stable.*

## Writing your own

What makes these prompts work, and what to keep in one of your own:

1. **Name Forge exactly, and give the install line.** "Forge" is a crowded name. Without `mollendorff-ai/forge` an AI may look for another one, and without the line it may spend minutes compiling.
2. **Say which files and which commands.** "Use `forge variance budget.yaml actual.yaml`" gets a precise answer; "analyse my budget" gets a guess at what Forge can do. `forge --help` and `forge <command> --help` list the rest, and an AI will read them if told to.
3. **Ask for the formula behind a number.** Every result in Forge comes from a formula in the YAML; asking to see it keeps the answer honest and shows you where to change it.
4. **Ask for one result checked another way.** The AI's own code, a hand calculation, a copy of the model with one input changed. Comparing with Forge's own documentation checks only that Forge ran.
5. **End with the YAML.** Every file, in full, with its name. That is what you keep: it opens in Studio, goes into Excel with `forge export`, and is the same model whoever calculates it next.
6. **Tell it what to do if it cannot run commands.** Otherwise an AI without a sandbox may describe results it never computed.

An agent that loads [Agent Skills](https://agentskills.io/specification) can carry all six in [the Forge skill](../skills/forge-modelling/SKILL.md), added once instead of pasted each time; [the guide's AI page](guide/ai.md#the-forge-skill) says where it goes.
