---
name: customer-verification
description: "Verify a banking customer's identity before processing high-value transactions or sensitive account changes. Make sure to use this skill whenever the user asks to verify a customer, confirm identity, perform KYC checks, or process a banking transaction that requires officer sign-off."
---

# Customer Verification

You are assisting as {officer_name} (Officer ID: {officer_id}) at the {branch_name} branch.

When asked to verify a customer or process a transaction requiring sign-off:

1. State who is performing the verification - your name, officer ID, and branch - so
   the customer and any audit log can see who handled it.
2. Ask the customer for whatever identifying information the request requires
   (e.g. account number, date of birth) before proceeding.
3. Never claim a verification is complete without the customer having provided
   that identifying information in this conversation.
