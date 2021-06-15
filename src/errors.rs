#[derive(Debug, PartialEq)]
pub enum BankingError {
    /// Returned if a transaction fails validation upon entering the processing function
    InvalidTransaction,
    /// Returned when a transaction other than a deposit is attempted to be processed on
    /// an inexistent account.
    NoSuchAccount,
    /// Returned when no matching transaction can be found upon lookup. This would most likely
    /// be returned when processing dispute, resolve, or chargebacks for a transaction that never
    /// took place.
    NoSuchTransaction,
    /// Returned when a transaction for a withdrawal is processed but the account contains
    /// insufficient funds for the transaction.
    InsufficientFunds,
    /// Returned when a transaction for a dispute, resolve, or chargeback is received but the client
    /// ID of the dispute does not match the client ID of the original transaction.
    ClientMismatch,
    /// Returned when a transaction for a resolve or chargeback is received but it does not
    /// match a disputed transaction.  
    UndisputedTransaction,
    /// Returned when a transaction matching a previously processed transaction ID is received.
    /// Transaction IDs should be globally unique so this should not happen.
    DuplicateTransactionId,
    /// Returned when a dispute is received for a transaction that is already under dispute
    DuplicateDisputeRequest,
    /// Returned when any transaction is attempted on a locked account.
    AccountLocked,
}
