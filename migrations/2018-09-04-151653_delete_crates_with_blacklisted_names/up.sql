DELETE FROM crates WHERE name IN (
    'abstract', 'alignof', 'as', 'become', 'box', 'break', 'const', 'continue', 'crate', 'do',
    'else', 'enum', 'extern', 'false', 'final', 'fn', 'for', 'if', 'impl', 'in', 'let', 'loop',
    'macro', 'match', 'mod', 'move', 'mut', 'offsetof', 'override', 'priv', 'proc', 'pub',
    'pure', 'ref', 'return', 'self', 'sizeof', 'static', 'struct', 'super', 'test', 'trait',
    'true', 'type', 'typeof', 'unsafe', 'unsized', 'use', 'virtual', 'where', 'while', 'yield'
)