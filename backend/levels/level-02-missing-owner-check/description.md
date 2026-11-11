# Level 2: The Wrong Signature

The vault from Level 1 now insists that the withdrawal is signed. Each vault
still records its owner, and only that owner should be able to withdraw.

Vaults are being drained anyway, and every one of those withdrawals carries a
valid signature.

**Your goal:** stop the exploit without breaking the owner's own withdrawals.
Edit the program, then submit. Tests will attack your fix.
