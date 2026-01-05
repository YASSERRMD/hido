import hido
import json

def header(text):
    print(f"\n{'='*50}\n{text}\n{'='*50}")

def main():
    print("🚀 HIDO Complete Python Bindings Example")

    # ---------------------------------------------------------
    # 1. Identity Management (DID)
    # ---------------------------------------------------------
    header("1. Identity Management (UAIL)")
    
    # Initialize Manager
    did_manager = hido.DIDManager()
    
    # Generate Agent Identity
    print("Generating Agent Identity...")
    did = did_manager.generate()
    print(f"✅ Generated DID: {did}")

    # Resolve DID Document
    print("\nResolving DID Document...")
    doc = did_manager.resolve(did)
    doc_json = json.loads(doc.to_json())
    print(f"✅ Resolved Document: {json.dumps(doc_json, indent=2)}")

    # Crypto Operations
    print("\nCrypto Operations (Sign/Verify)...")
    message = b"Hello HIDO Agents!"
    signature = did_manager.sign(did, message)
    print(f"✅ Signed Message: {message.decode()}")
    print(f"   Signature (hex): {signature.hex()[:32]}...")
    
    valid = did_manager.verify(did, message, signature)
    print(f"✅ Signature Verified: {valid}")

    # ---------------------------------------------------------
    # 2. Semantic Intent (ICC)
    # ---------------------------------------------------------
    header("2. Semantic Intent (ICC)")
    
    print("Creating Complex Intent...")
    intent = hido.Intent("analyze_dataset", "finance")
    
    # Fluent API
    intent = (intent
        .set_target("s3://data-lake/financial-records.csv")
        .set_priority(2)  # High Priority
        .add_param("format", "parquet")
        .add_param("compression", "snappy")
    )
    
    intent_json = json.loads(intent.to_json())
    print(f"✅ Intent Created:")
    print(f"   ID: {intent.get_id}")
    print(f"   Action: {intent.get_action}")
    print(f"   Details: {json.dumps(intent_json, indent=2)}")

    # ---------------------------------------------------------
    # 3. Flexible Audit Layer (BAL)
    # ---------------------------------------------------------
    header("3. Flexible Audit Layer")
    
    try:
        print("Initializing Blockchain Backend...")
        audit = hido.AuditBackend.create_blockchain()
        print(f"✅ Backend Initialized: {audit.backend_type().upper()}")

        print("\nRecording Audit Entry...")
        entry_id = audit.record(
            actor=did,
            action="accessed_secure_data",
            target="s3://data-lake/financial-records.csv"
        )
        print(f"✅ Audit Entry Recorded!")
        print(f"   Entry ID: {entry_id}")
        
    except Exception as e:
        print(f"❌ Audit Error: {e}")

    header("Example Complete ✓")

if __name__ == "__main__":
    main()
