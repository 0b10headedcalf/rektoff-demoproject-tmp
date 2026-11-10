import {
  completeAnyWord,
  snippetCompletion,
  type Completion,
  type CompletionContext,
  type CompletionResult,
} from "@codemirror/autocomplete";
import { rustLanguage } from "@codemirror/lang-rust";
import { syntaxTree } from "@codemirror/language";

// This is static completion, not rust-analyzer: it knows the vocabulary of
// Rust and Anchor but nothing about the types in the user's program.

const KEYWORDS = [
  "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum",
  "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move",
  "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait",
  "true", "type", "unsafe", "use", "where", "while",
];

const TYPES = [
  "u8", "u16", "u32", "u64", "u128", "usize", "i8", "i16", "i32", "i64", "i128", "isize",
  "bool", "str", "String", "Vec", "Option", "Some", "None", "Result", "Ok", "Err", "Box",
];

/** Anchor / Solana names. Note that this list offers `Signer`, `has_one`, etc. */
const ANCHOR: [label: string, detail: string][] = [
  ["Account", "Account<'info, T>"],
  ["AccountInfo", "AccountInfo<'info>"],
  ["AccountLoader", "AccountLoader<'info, T>"],
  ["Accounts", "derive"],
  ["Context", "Context<T>"],
  ["CpiContext", "anchor"],
  ["InitSpace", "derive"],
  ["Program", "Program<'info, T>"],
  ["Pubkey", "solana"],
  ["Signer", "Signer<'info>"],
  ["System", "program"],
  ["SystemAccount", "SystemAccount<'info>"],
  ["UncheckedAccount", "UncheckedAccount<'info>"],
  ["anchor_lang", "crate"],
  ["prelude", "anchor_lang"],
  ["system_program", "module"],
  ["is_signer", "field"],
  ["lamports", "field"],
  ["key", "fn"],
  ["to_account_info", "fn"],
  ["checked_add", "fn"],
  ["checked_sub", "fn"],
  ["checked_mul", "fn"],
  ["ok_or", "fn"],
  // #[account(...)] constraints
  ["mut", "constraint"],
  ["init", "constraint"],
  ["payer", "constraint"],
  ["space", "constraint"],
  ["seeds", "constraint"],
  ["bump", "constraint"],
  ["has_one", "constraint"],
  ["constraint", "constraint"],
  ["address", "constraint"],
  ["close", "constraint"],
  ["signer", "constraint"],
];

const SNIPPETS: Completion[] = [
  snippetCompletion("fn ${name}(${params}) -> ${Result<()>} {\n\t${}\n}", {
    label: "fn",
    detail: "function",
  }),
  snippetCompletion("#[derive(Accounts)]\npub struct ${Name}<'info> {\n\t${}\n}", {
    label: "accounts",
    detail: "#[derive(Accounts)] struct",
  }),
  snippetCompletion("#[account(${mut})]\npub ${name}: Account<'info, ${Type}>,", {
    label: "account",
    detail: "#[account(...)] field",
  }),
  snippetCompletion("require!(${condition}, ${ErrorCode::Variant});", {
    label: "require!",
    detail: "macro",
  }),
  snippetCompletion("require_keys_eq!(${a}, ${b}, ${ErrorCode::Variant});", {
    label: "require_keys_eq!",
    detail: "macro",
  }),
  snippetCompletion('msg!("${message}");', { label: "msg!", detail: "macro" }),
  snippetCompletion("err!(${ErrorCode::Variant})", { label: "err!", detail: "macro" }),
  snippetCompletion("#[error_code]\npub enum ${ErrorCode} {\n\t#[msg(\"${message}\")]\n\t${Variant},\n}", {
    label: "error_code",
    detail: "#[error_code] enum",
  }),
  snippetCompletion("match ${value} {\n\t${pattern} => ${},\n}", {
    label: "match",
    detail: "expression",
  }),
  snippetCompletion("if let ${Some(x)} = ${value} {\n\t${}\n}", {
    label: "if let",
    detail: "expression",
  }),
];

const OPTIONS: Completion[] = [
  ...SNIPPETS.map((snippet) => ({ ...snippet, type: "snippet", boost: 1 })),
  ...KEYWORDS.map((label) => ({ label, type: "keyword" })),
  ...TYPES.map((label) => ({ label, type: "type" })),
  ...ANCHOR.map(([label, detail]) => ({ label, detail, type: "class" })),
];

const KNOWN = new Set(OPTIONS.map((option) => option.label));
const NO_COMPLETE = new Set(["LineComment", "BlockComment", "String", "RawString", "Char"]);

function inCommentOrString(context: CompletionContext) {
  return NO_COMPLETE.has(syntaxTree(context.state).resolveInner(context.pos, -1).name);
}

function rustVocabulary(context: CompletionContext): CompletionResult | null {
  if (inCommentOrString(context)) return null;
  const word = context.matchBefore(/\w+!?/);
  if (!word && !context.explicit) return null;
  return { from: word?.from ?? context.pos, options: OPTIONS, validFor: /^\w*!?$/ };
}

/** Identifiers already in the file, ranked below the vocabulary above. */
async function documentWords(context: CompletionContext): Promise<CompletionResult | null> {
  if (inCommentOrString(context)) return null;
  const result = await completeAnyWord(context);
  if (!result) return null;
  return {
    ...result,
    options: result.options
      // Also drop bare macro names ("require") already offered as "require!".
      .filter((option) => !KNOWN.has(option.label) && !KNOWN.has(`${option.label}!`))
      .map((option) => ({ ...option, type: "variable", boost: -10 })),
  };
}

/** Plugs into the `autocompletion()` that `basicSetup` already enables. */
export const rustCompletions = [
  rustLanguage.data.of({ autocomplete: rustVocabulary }),
  rustLanguage.data.of({ autocomplete: documentWords }),
];
