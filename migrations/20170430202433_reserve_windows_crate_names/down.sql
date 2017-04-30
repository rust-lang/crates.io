DELETE FROM reserved_crate_names WHERE name IN
    ('nul', 'con', 'prn', 'aux', 'com1', 'com2', 'com3', 'com4',
    'com5', 'com6', 'com7', 'com8', 'com9', 'lpt1', 'lpt2',
    'lpt3', 'lpt4', 'lpt5', 'lpt6', 'lpt7', 'lpt8', 'lpt9');
