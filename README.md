
# MEC
- Using threads over async. One thread per account blocking for bg updates. 
- Qt frontend, SQLite db

## CLI
- `mec list x`: List x most recent msg header and uid -> print or err
- `mec listm`: lists possible mailboxes -> print or err
- `mec select x`: selects mailbox x -> confirm or err
- `mec read x`: print msg with x uid -> print or err
- `mec flag x f`: toggle flag f on msg x -> confirm or err

