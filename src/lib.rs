use metashrew_support::index_pointer::KeyValuePointer;
use metashrew_support::compat::to_arraybuffer_layout;
use metashrew_support::utils::consensus_decode;

use alkanes_runtime::{
  declare_alkane, message::MessageDispatch, storage::StoragePointer, token::Token,
  runtime::AlkaneResponder
};

use alkanes_support::{
  id::AlkaneId,
  parcel::AlkaneTransfer, response::CallResponse,
  utils::overflow_error
};

use bitcoin::hashes::Hash;
use bitcoin::{Txid, Transaction};

use anyhow::{anyhow, Result};
use std::sync::Arc;

// We could validate monkeys ids against the collection contract 2:614, but we cbf. Save fuel.
mod monkey_ids;
use monkey_ids::MONKEY_IDS;

mod monkey_image;
use monkey_image::MONKEY_IMAGE;

const MONKEY_BLOCK: u128 = 0x2;

const BANANA_PER_MINT: u128 = 25_500;
const BANANA_MINT: u128 = 200_000;
const BANANA_CAP: u128 = BANANA_MINT * BANANA_PER_MINT;
const MONKEY_PER_BURN: u128 = 1000;

#[derive(Default)]
pub struct BananaMint(());

impl AlkaneResponder for BananaMint {}

#[derive(MessageDispatch)]
enum BananaMintMessage {
  #[opcode(0)]
  Initialize,

  #[opcode(42)]
  MonkeyToBanana,

  #[opcode(77)]
  MintTokens,

  #[opcode(99)]
  #[returns(String)]
  GetName,

  #[opcode(100)]
  #[returns(String)]
  GetSymbol,

  #[opcode(101)]
  #[returns(u128)]
  GetTotalSupply,
  
  #[opcode(102)]
  #[returns(u128)]
  GetCap,

  #[opcode(103)]
  #[returns(u128)]
  GetMinted,

  #[opcode(104)]
  #[returns(u128)]
  GetValuePerMint,

  #[opcode(1000)]
  #[returns(Vec<u8>)]
  GetData,

  #[opcode(2000)]
  #[returns(u128)]
  GetMonkeyStackCount,

  #[opcode(2001)]
  #[returns(Vec<Vec<u8>>)]
  GetMonkeyStack,

  #[opcode(2002)]
  #[returns(String)]
  GetMonkeyStackJson,
}

impl Token for BananaMint {
  fn name(&self) -> String {
    return String::from("banana")
  }

  fn symbol(&self) -> String {
    return String::from("banana");
  }
}

impl BananaMint {
  fn initialize(&self) -> Result<CallResponse> {
    self.observe_initialization()?;
    let context = self.context()?;

    let response = CallResponse::forward(&context.incoming_alkanes);
    Ok(response)
  }

  fn get_name(&self) -> Result<CallResponse> {
    let context = self.context()?;
    let mut response = CallResponse::forward(&context.incoming_alkanes);

    response.data = self.name().into_bytes();

    Ok(response)
  }

  fn get_symbol(&self) -> Result<CallResponse> {
    let context = self.context()?;
    let mut response = CallResponse::forward(&context.incoming_alkanes);

    response.data = self.symbol().into_bytes();

    Ok(response)
  }

  fn get_total_supply(&self) -> Result<CallResponse> {
    let context = self.context()?;
    let mut response = CallResponse::forward(&context.incoming_alkanes);

    response.data = self.total_supply().to_le_bytes().to_vec();

    Ok(response)
  }

  fn get_cap(&self) -> Result<CallResponse> {
    let context = self.context()?;
    let mut response = CallResponse::forward(&context.incoming_alkanes);

    response.data = BANANA_CAP.to_le_bytes().to_vec();

    Ok(response)
  }

  fn get_minted(&self) -> Result<CallResponse> {
    let context = self.context()?;
    let mut response = CallResponse::forward(&context.incoming_alkanes);

    response.data = self.instances_count().to_le_bytes().to_vec();

    Ok(response)
  }

  fn get_value_per_mint(&self) -> Result<CallResponse> {
    let context = self.context()?;
    let mut response = CallResponse::forward(&context.incoming_alkanes);

    response.data = BANANA_PER_MINT.to_le_bytes().to_vec();

    Ok(response)
  }

  fn get_data(&self) -> Result<CallResponse> {
    let context = self.context()?;
    let mut response = CallResponse::forward(&context.incoming_alkanes);

    response.data = MONKEY_IMAGE.to_vec();

    Ok(response)
  }

  fn total_supply_pointer(&self) -> StoragePointer {
    StoragePointer::from_keyword("/total_supply")
  }

  fn total_supply(&self) -> u128 {
    self.total_supply_pointer().get_value::<u128>()
  }

  fn set_total_supply(&self, v: u128) {
    self.total_supply_pointer().set_value::<u128>(v);
  }

  fn increase_total_supply(&self, v: u128) -> Result<()> {
    self.set_total_supply(overflow_error(self.total_supply().checked_add(v))?);
    Ok(())
  }

  fn decrease_total_supply(&self, v: u128) -> Result<()> {
    self.set_total_supply(overflow_error(self.total_supply().checked_sub(v))?);
    Ok(())
  }

  fn is_valid_monkey(&self, id: &AlkaneId) -> Result<bool> {
    Ok(id.block == MONKEY_BLOCK && MONKEY_IDS.contains(&id.tx))
  }

  fn monkey_to_banana(&self) -> Result<CallResponse> {
    let context = self.context()?;
    let txid = self.transaction_id()?;

    // Enforce one mint per transaction
    if self.has_tx_hash(&txid) {
      return Err(anyhow!("Transaction already used for mint"));
    }
    
    if context.incoming_alkanes.0.is_empty() {
      return Err(anyhow!("Must send at least 1000 Monkey to mint"));
    }

    self.add_tx_hash(&txid)?;

    let mut response = CallResponse::default();
    let mut total_banana = 0u128;

    for alkane in context.incoming_alkanes.0.iter() {
      if !self.is_valid_monkey(&alkane.id)? {
        return Err(anyhow!("Invalid Monkey ID"));
      }

      // self.add_instance(&alkane.id)?;
      let transfer = context.incoming_alkanes.0[0].clone();
      if transfer.value != MONKEY_PER_BURN {
        return Err(anyhow!(
          "Not correct $monkey supplied to mint"
        ));
      }
      let value = (self.block()[67] ^ txid.as_byte_array()[31]) as u128 * 100;


      total_banana = total_banana.checked_add(value)
        .ok_or_else(|| anyhow!("Banana amount overflow"))?;
    }

    
    let new_total = self.total_supply().checked_add(total_banana)
    .ok_or_else(|| anyhow!("Banana total supply overflow"))?;

    if new_total > BANANA_CAP {
      return Err(anyhow!("Banana cap exceeded"));
    }

    self.increase_total_supply(total_banana)?;

    response.alkanes.0.push(AlkaneTransfer {
      id: context.myself.clone(),
      value: total_banana,
    }); 

    Ok(response)
  }

  fn mint_tokens(&self) -> Result<CallResponse> {
    return Err(anyhow!("Minting not implemented"));
  }

  fn get_monkey_stack_count(&self) -> Result<CallResponse> {
    let context = self.context()?;
    let mut response = CallResponse::forward(&context.incoming_alkanes);

    response.data = self.instances_count().to_le_bytes().to_vec();

    Ok(response)
  }

  fn get_monkey_stack(&self) -> Result<CallResponse> {
    let context = self.context()?;
    let mut response = CallResponse::forward(&context.incoming_alkanes);

    let count = self.instances_count();
    let mut monkey_ids = Vec::new();

    for i in 0..count {
      let instance_id = self.lookup_instance(i)?;
      let mut bytes = Vec::with_capacity(32);
      bytes.extend_from_slice(&instance_id.block.to_le_bytes());
      bytes.extend_from_slice(&instance_id.tx.to_le_bytes());
      monkey_ids.push(bytes);
    }

    let mut flattened = Vec::new();
    for bytes in monkey_ids {
      flattened.extend(bytes);
    }

    response.data = flattened;
    Ok(response)
  }

  fn get_monkey_stack_json(&self) -> Result<CallResponse> {
    let context = self.context()?;
    let mut response = CallResponse::forward(&context.incoming_alkanes);

    let count = self.instances_count();
    let mut monkey_ids = Vec::new();

    for i in 0..count {
      let instance_id = self.lookup_instance(i)?;
      monkey_ids.push(format!("{}:{}", instance_id.block, instance_id.tx));
    }

    response.data = serde_json::to_string(&monkey_ids)?.into_bytes();
    Ok(response)
  }

  fn instances_pointer(&self) -> StoragePointer {
    StoragePointer::from_keyword("/instances")
  }

  fn instances_count(&self) -> u128 {
    self.instances_pointer().get_value::<u128>()
  }

  fn set_instances_count(&self, count: u128) {
    self.instances_pointer().set_value::<u128>(count);
  }

  fn add_instance(&self, instance_id: &AlkaneId) -> Result<u128> {
    let count = self.instances_count();
    let new_count = count.checked_add(1)
      .ok_or_else(|| anyhow!("instances count overflow"))?;

    let mut bytes = Vec::with_capacity(32);
    bytes.extend_from_slice(&instance_id.block.to_le_bytes());
    bytes.extend_from_slice(&instance_id.tx.to_le_bytes());

    let bytes_vec = new_count.to_le_bytes().to_vec();
    let mut instance_pointer = self.instances_pointer().select(&bytes_vec);
    instance_pointer.set(Arc::new(bytes));
    
    self.set_instances_count(new_count);
    
    Ok(new_count)
  }

  fn pop_instance(&self) -> Result<AlkaneId> {
    let count = self.instances_count();

    let new_count = count.checked_sub(1)
      .ok_or_else(|| anyhow!("instances count underflow"))?;

    let instance_id = self.lookup_instance(count - 1)?;
    
    // Remove the instance by setting it to empty
    let bytes_vec = count.to_le_bytes().to_vec();
    let mut instance_pointer = self.instances_pointer().select(&bytes_vec);
    instance_pointer.set(Arc::new(Vec::new()));
    
    self.set_instances_count(new_count);
    
    Ok(instance_id)
  }

  fn lookup_instance(&self, index: u128) -> Result<AlkaneId> {
    let bytes_vec = (index + 1).to_le_bytes().to_vec();
    let instance_pointer = self.instances_pointer().select(&bytes_vec);
    
    let bytes = instance_pointer.get();
    if bytes.len() != 32 {
      return Err(anyhow!("Invalid instance data length"));
    }

    let block_bytes = &bytes[..16];
    let tx_bytes = &bytes[16..];

    let block = u128::from_le_bytes(block_bytes.try_into().unwrap());
    let tx = u128::from_le_bytes(tx_bytes.try_into().unwrap());

    Ok(AlkaneId { block, tx })
  }

  fn transaction_id(&self) -> Result<Txid> {
    Ok(
      consensus_decode::<Transaction>(&mut std::io::Cursor::new(self.transaction()))?
        .compute_txid(),
    )
  }

  fn has_tx_hash(&self, txid: &Txid) -> bool {
    StoragePointer::from_keyword("/tx-hashes/")
      .select(&txid.as_byte_array().to_vec())
      .get_value::<u8>()
      == 1
  }

  fn add_tx_hash(&self, txid: &Txid) -> Result<()> {
    StoragePointer::from_keyword("/tx-hashes/")
      .select(&txid.as_byte_array().to_vec())
      .set_value::<u8>(0x01);

    Ok(())
  }
}

declare_alkane! {
  impl AlkaneResponder for BananaMint {
    type Message = BananaMintMessage;
  }
}
