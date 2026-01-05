import hido
import asyncio

async def main():
    print("🚀 HIDO Python Example")
    
    # 1. Identity Management
    print("\n[1] Generating Agent Identity...")
    did_manager = hido.DIDManager()
    did = did_manager.generate()
    print(f"    Generated DID: {did}")

    # 2. Intent Creation
    print("\n[2] Creating Intent...")
    intent = hido.Intent("analyze_data", "finance")
    print(f"    Intent created with ID: {intent.get_id}")
    print(f"    Action: {intent.get_action}")

    print("\n✅ Success! HIDO bindings are working.")

if __name__ == "__main__":
    # Note: real async support requires more complex binding setup
    # This example demonstrates the synchronous wrappers we built
    try:
        import asyncio
        # For now, our bindings block internally so we don't need await here
        # but the structure is ready for async
        
        # Identity Management
        print("🚀 HIDO Python Example")
        print("\n[1] Generating Agent Identity...")
        did_manager = hido.DIDManager()
        did = did_manager.generate()
        print(f"    Generated DID: {did}")

        # Intent
        print("\n[2] Creating Intent...")
        intent = hido.Intent("intent-123", "agent-007")
        print(f"    Intent created with ID: {intent.get_id}")
        
    except ImportError:
        print("❌ Error: 'hido' module not found.")
        print("   Make sure you have built the bindings using 'maturin develop'")
