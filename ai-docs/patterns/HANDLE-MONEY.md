# Pattern: Handle Money

## When to Use
- Working with any monetary amounts in the system
- Storing prices, fees, or financial values
- Performing calculations on monetary values
- Converting between major units (dollars) and minor units (cents)
- Displaying monetary values in API responses

## Quick Example
```rust
use common_types::AmountType;
use common_enums::Currency;
use common_utils::types::MinorUnit;

// Create from major units (e.g., dollars)
let price = AmountType::from_major_unit(99.99, Currency::USD)?; // $99.99

// Create from minor units (e.g., cents)
let fee = AmountType::new(MinorUnit::new(250), Currency::USD);  // $2.50

// Currency-safe arithmetic
let total = (price + fee)?; // Fails if currencies don't match
```

## Full Implementation
```rust
// In API models - use AmountType directly
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PaymentRequest {
    pub amount: AmountType,
    pub description: String,
}

// For staging entries - separate amount and currency
#[derive(Serialize, Deserialize, ToSchema)]
pub struct StagingEntryCreateRequest {
    pub amount: f64,      // Major units (dollars)
    pub currency: Currency,
}

// In database models - store as MinorUnit + Currency
#[derive(Queryable, Selectable)]
#[diesel(table_name = payment)]
pub struct Payment {
    pub id: String,
    pub amount: MinorUnit,    // Always minor units in DB
    pub currency: Currency,
    pub created_at: time::PrimitiveDateTime,
}

// Convert API request to DB model
impl From<PaymentRequest> for PaymentNew {
    fn from(req: PaymentRequest) -> Self {
        Self {
            id: {
                let id = nanoid::nanoid!(10);
                format!("payment_{id}")
            },
            amount: req.amount.value(),        // Extract MinorUnit
            currency: req.amount.currency(),   // Extract Currency
        }
    }
}

// Convert staging entry request to DB model
impl From<StagingEntryCreateRequest> for StagingEntryNew {
    fn from(req: StagingEntryCreateRequest) -> Self {
        // Convert major units to minor units
        let amount_type = AmountType::from_major_unit(req.amount, req.currency)
            .expect("Invalid amount");
            
        Self {
            id: {
                let id = nanoid::nanoid!(10);
                format!("entry_{id}")
            },
            amount: amount_type.value(),
            currency: req.currency,
        }
    }
}

// Convert DB model to API response
impl From<Payment> for PaymentResponse {
    fn from(payment: Payment) -> Self {
        Self {
            id: payment.id,
            // Reconstruct AmountType for response
            amount: AmountType::new(payment.amount, payment.currency),
            created_at: payment.created_at,
        }
    }
}

// Safe arithmetic operations
pub fn calculate_total_with_tax(
    subtotal: AmountType,
    tax_rate: f64,
) -> Result<AmountType, error_stack::Report<CalculationError>> {
    // Calculate tax amount
    let tax = subtotal.multiply_by_fraction(tax_rate)
        .change_context(CalculationError::InvalidTaxRate)?;
    
    // Add tax to subtotal (currency-safe)
    let total = (subtotal + tax)
        .change_context(CalculationError::CurrencyMismatch)?;
    
    Ok(total)
}
```

## Key Points
- **Never use f64 for money** - Always use AmountType or MinorUnit
- **Database stores minor units** - Convert at API boundaries
- **AmountType for API models** - Provides currency safety and formatting
- **Separate fields for staging entries** - When amount and currency come separately
- **Currency-safe arithmetic** - Operations fail if currencies don't match
- **Display formatting** - AmountType implements Display (e.g., "99.99 USD")

## Common Mistakes
- Using floating point (f64) for monetary calculations
- Storing major units (dollars) in database instead of minor units (cents)
- Mixing currencies in arithmetic operations
- Not handling currency conversion errors
- Forgetting to validate amount ranges

## Related Patterns
- [ERROR-HANDLING](ERROR-HANDLING.md) - For handling conversion errors
- [DATABASE-QUERY](DATABASE-QUERY.md) - For storing monetary values
- [ADD-ENDPOINT](ADD-ENDPOINT.md) - For money in API requests

## Gotchas to Remember
- All amounts in database are BIGINT storing minor units
- Currency enum is stored separately from amount
- AmountType arithmetic returns Result - handle errors
- Different currencies have different minor unit scales (JPY has none)
- Always validate amounts are positive where required