# Averqun Bootstrap Grammar

This file records the original bootstrap grammar. The parser also accepts the experimental alternative syntax described in [the language specification](language-specification.md#10-experimental-alternative-syntax). It is not the final Averqun grammar.

```ebnf
program       = { module_decl | function } ;
module_decl   = "module" identifier [ ";" ] ;
function      = "fn" identifier "(" [ parameters ] ")" [ "->" identifier ] block ;
parameters    = identifier { "," identifier } ;
block         = "{" { statement } "}" ;
statement     = variable | return_stmt | if_stmt | block | assignment | expression ;
variable      = ("let" | "var") identifier [ ":" identifier ] "=" expression [ ";" ] ;
return_stmt   = "return" [ expression ] [ ";" ] ;
if_stmt       = "if" expression block [ "else" (block | if_stmt) ] ;
assignment    = identifier "=" expression [ ";" ] ;
expression    = logical_or ;
logical_or    = logical_and { "||" logical_and } ;
logical_and   = equality { "&&" equality } ;
equality      = comparison { ("==" | "!=") comparison } ;
comparison    = term { ("<" | "<=" | ">" | ">=") term } ;
term          = factor { ("+" | "-") factor } ;
factor        = unary { ("*" | "/" | "%") unary } ;
unary         = ("!" | "-") unary | primary ;
primary       = number | string | "true" | "false" | "nil"
              | identifier [ "(" [ arguments ] ")" ]
              | "print" "(" [ arguments ] ")" | "(" expression ")" ;
arguments     = expression { "," expression } ;
```

Types in the bootstrap are inferred from primitive literals. The parser currently consumes but does not retain local/return type annotations. Functions are declarations only; there are no closures, structs, enums, traits, imports, generics, loops, or pattern matching yet.
