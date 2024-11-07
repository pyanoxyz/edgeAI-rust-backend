
#### response from the planner stage
```yaml
steps:
  - step_number: "1"
    heading: "Create a new directory for the smart contract project."
    action: create_directory
    details:
      directory: multi_signature_wallet
  - step_number: "2"
    heading: "Navigate to the newly created directory."
    action: system_command
    details:
      command: cd multi_signature_wallet
  - step_number: "3"
    heading: "Create a new Solidity file for the smart contract."
    action: create_file
    details:
      filename: MultiSignatureWallet.sol
  - step_number: "4"
    heading: "Edit the newly created Solidity file with the multi-signature wallet logic."
    action: edit_file
    details:
      filename: MultiSignatureWallet.sol
  - step_number: "5"
    heading: "Install OpenZeppelin contracts for security best practices and utilities."
    action: install_dependency
    details:
      package_name: openzeppelin-solidity@4.0.1
  - step_number: "6"
    heading: "Write the smart contract code in MultiSignatureWallet.sol to include flexible signers, threshold approval logic, non-repudiation measures, and efficient execution."
    action: edit_file
    details:
      filename: MultiSignatureWallet.sol
  - step_number: "7"
    heading: "Compile the Solidity file using Truffle or Hardhat for Ethereum development environments."
    action: system_command
    details:
      command: truffle compile
  - step_number: "8"
    heading: "Deploy the smart contract to an Ethereum network (e.g., Ropsten, Rinkeby) via a deployment script created with Truffle or Hardhat."
    action: create_file
    details:
      filename: deploy.js
  - step_number: "9"
    heading: "Edit the newly created deployment script file to include logic for deploying the MultiSignatureWallet contract on the Ethereum network."
    action: edit_file
    details:
      filename: deploy.js
  - step_number: "10"
    heading: "Run the deployment script using Truffle or Hardhat, specifying the desired configuration (e.g., account addresses and threshold)."
    action: system_command
    details:
      command: truffle migrate --network ropsten
  - step_number: "11"
    heading: "Verify that transactions can only be executed if a specified minimum number of members approve them."
    action: run_tests
```
#### Get request to fetch all the steps for the pair programmer
```
r = requests.get(f"http://localhost:52556/pair-programmer/steps/{pair_programming_id}")

Output:
{'steps': [{'action': 'create_directory',
   'chat': '[]',
   'details': {'directory': 'multi_signature_wallet'},
   'executed': False,
   'heading': '"Create a new directory for the smart contract project."',
   'response': '',
   'session_id': '91deac5d-d97e-4af4-b953-cfdfebf6138b',
   'step_id': '68c308fa-932c-405d-b6cf-bc1a35aba943_1',
   'timestamp': '2024-11-06T14:20:53.239233+00:00',
   'user_id': 'user_id'},
  {'action': 'system_command',
   'chat': '[]',
   'details': {'command': 'cd multi_signature_wallet'},
   'executed': False,
   'heading': '"Navigate to the newly created directory."',
   'response': '',
   'session_id': '91deac5d-d97e-4af4-b953-cfdfebf6138b',
   'step_id': '68c308fa-932c-405d-b6cf-bc1a35aba943_2',
   'timestamp': '2024-11-06T14:20:53.239233+00:00',
   'user_id': 'user_id'},
  {'action': 'create_file',
   'chat': '[]',
   'details': {'filename': 'MultiSignatureWallet.sol'},
   'executed': False,
   'heading': '"Create a new Solidity file for the smart contract."',
   'response': '',
   'session_id': '91deac5d-d97e-4af4-b953-cfdfebf6138b',
   'step_id': '68c308fa-932c-405d-b6cf-bc1a35aba943_3',
   'timestamp': '2024-11-06T14:20:53.239233+00:00',
   'user_id': 'user_id'},
  {'action': 'edit_file',
   'chat': '[]',
   'details': {'filename': 'MultiSignatureWallet.sol'},
   'executed': False,
   'heading': '"Edit the newly created Solidity file with the multi-signature wallet logic."',
   'response': '',
   'session_id': '91deac5d-d97e-4af4-b953-cfdfebf6138b',
   'step_id': '68c308fa-932c-405d-b6cf-bc1a35aba943_4',
   'timestamp': '2024-11-06T14:20:53.239233+00:00',
   'user_id': 'user_id'},
  {'action': 'install_dependency',
   'chat': '[]',
   'details': {'package_name': 'openzeppelin-solidity@4.0.1'},
   'executed': False,
   'heading': '"Install OpenZeppelin contracts for security best practices and utilities."',
   'response': '',
   'session_id': '91deac5d-d97e-4af4-b953-cfdfebf6138b',
   'step_id': '68c308fa-932c-405d-b6cf-bc1a35aba943_5',
   'timestamp': '2024-11-06T14:20:53.239233+00:00',
   'user_id': 'user_id'},
  {'action': 'edit_file',
   'chat': '[]',
   'details': {'filename': 'MultiSignatureWallet.sol'},
   'executed': False,
   'heading': '"Write the smart contract code in MultiSignatureWallet.sol to include flexible signers, threshold approval logic, non-repudiation measures, and efficient execution."',
   'response': '',
   'session_id': '91deac5d-d97e-4af4-b953-cfdfebf6138b',
   'step_id': '68c308fa-932c-405d-b6cf-bc1a35aba943_6',
   'timestamp': '2024-11-06T14:20:53.239233+00:00',
   'user_id': 'user_id'},
  {'action': 'system_command',
   'chat': '[]',
   'details': {'command': 'truffle compile'},
   'executed': False,
   'heading': '"Compile the Solidity file using Truffle or Hardhat for Ethereum development environments."',
   'response': '',
   'session_id': '91deac5d-d97e-4af4-b953-cfdfebf6138b',
   'step_id': '68c308fa-932c-405d-b6cf-bc1a35aba943_7',
   'timestamp': '2024-11-06T14:20:53.239233+00:00',
   'user_id': 'user_id'},
  {'action': 'create_file',
   'chat': '[]',
   'details': {'filename': 'deploy.js'},
   'executed': False,
   'heading': '"Deploy the smart contract to an Ethereum network (e.g., Ropsten, Rinkeby) via a deployment script created with Truffle or Hardhat."',
   'response': '',
   'session_id': '91deac5d-d97e-4af4-b953-cfdfebf6138b',
   'step_id': '68c308fa-932c-405d-b6cf-bc1a35aba943_8',
   'timestamp': '2024-11-06T14:20:53.239233+00:00',
   'user_id': 'user_id'},
  {'action': 'edit_file',
   'chat': '[]',
   'details': {'filename': 'deploy.js'},
   'executed': False,
   'heading': '"Edit the newly created deployment script file to include logic for deploying the MultiSignatureWallet contract on the Ethereum network."',
   'response': '',
   'session_id': '91deac5d-d97e-4af4-b953-cfdfebf6138b',
   'step_id': '68c308fa-932c-405d-b6cf-bc1a35aba943_9',
   'timestamp': '2024-11-06T14:20:53.239233+00:00',
   'user_id': 'user_id'},
  {'action': 'system_command',
   'chat': '[]',
   'details': {'command': 'truffle migrate --network ropsten'},
   'executed': False,
   'heading': '"Run the deployment script using Truffle or Hardhat, specifying the desired configuration (e.g., account addresses and threshold)."',
   'response': '',
   'session_id': '91deac5d-d97e-4af4-b953-cfdfebf6138b',
   'step_id': '68c308fa-932c-405d-b6cf-bc1a35aba943_10',
   'timestamp': '2024-11-06T14:20:53.239233+00:00',
   'user_id': 'user_id'},
  {'action': 'run_tests',
   'chat': '[]',
   'details': {},
   'executed': False,
   'heading': '"Verify that transactions can only be executed if a specified minimum number of members approve them."',
   'response': '',
   'session_id': '91deac5d-d97e-4af4-b953-cfdfebf6138b',
   'step_id': '68c308fa-932c-405d-b6cf-bc1a35aba943_11',
   'timestamp': '2024-11-06T14:20:53.239233+00:00',
   'user_id': 'user_id'}]}
```

#### Execute a step
```
r = requests.post("http://localhost:52556/pair-programmer/steps/execute", json={"pair_programmer_id": pair_programming_id, "step_number": "4"}

Ouput = 
"solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.0;
import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/cryptography/MerkleProof.sol";
contract MultiSignatureWallet is Ownable {
    uint256 public requiredSignatures;
    mapping(address => bool) private signers;
    address[] private signerList;
    event SignerAdded(address indexed account);
    event SignerRemoved(address indexed account);
    constructor(uint256 _requiredSignatures, address[] memory initialOwners) Ownable() {
        require(_requiredSignatures > 0 && _requiredSignatures <= initialOwners.length, "Invalid number of required signatures");

        for (uint i = 0; i < initialOwners.length; i++) {
            addOwner(initialOwners[i]);
        }
    }
    function getRequiredSignatures() public view returns(uint256) { return requiredSignatures;}

    modifier onlyWallet() {
        require(isApproved(msg.sender), "Not authorized to perform this action");
        _;
    }
"
```

#### chat with a step
```
r = requests.post("http://localhost:52556/pair-programmer/steps/chat", json={
            "pair_programmer_id": pair_programming_id, 
            "step_number": "11", 
            "prompt": "Change this to an instruction to write scripts in typescript that can test the multisignature wallet"}, stream=True)

If the step hasnt been executed, the every chat request will change the heading of the task bas3ed on the whole task and the prompt
If the step has been executed, the every chat request will change the response of the task based on the whole task and the prompt
```