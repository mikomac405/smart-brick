import 'dart:convert';
import 'package:flutter/material.dart';
import 'package:flutter_blue_plus/flutter_blue_plus.dart';
import 'package:flutter_joystick/flutter_joystick.dart';

void main() {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Smart Brick Control',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.blue),
        useMaterial3: true,
      ),
      home: const ControllerScreen(),
    );
  }
}

class ControllerScreen extends StatefulWidget {
  const ControllerScreen({super.key});

  @override
  State<ControllerScreen> createState() => _ControllerScreenState();
}

class _ControllerScreenState extends State<ControllerScreen> {
  // Target MAC address of the smart brick BLE device
  final String deviceId = '30:45:11:EA:34:73';
  final myCustomCommandController = TextEditingController();
  
  BluetoothDevice? _device;
  BluetoothCharacteristic? _writeCharacteristic;

  String _connectionState = 'Disconnected';
  String _lastCommand = 'S';

  @override
  void initState() {
    super.initState();
    // Try to connect to the device automatically on startup
    _connectToDevice();
  }

  Future<void> _connectToDevice() async {
    setState(() {
      _connectionState = 'Connecting...';
    });

    try {
      _device = BluetoothDevice.fromId(deviceId);
      
      // Connect to the device (with autoConnect false to attempt immediate connection)
      await _device!.connect(autoConnect: false, license: License.nonprofit);

      setState(() {
        _connectionState = 'Discovering Services...';
      });

      // Discover services on the connected device
      List<BluetoothService> services = await _device!.discoverServices();
      for (BluetoothService service in services) {
        if (service.uuid == Guid("FFE0")) {
          for (BluetoothCharacteristic char in service.characteristics) {
            if (char.uuid == Guid("FFE1")) {
              _writeCharacteristic = char;
              break;
            }
          }
        }
      }

      setState(() {
        if (_writeCharacteristic != null) {
          _connectionState = 'Connected & Ready';
          // Send initial Stop command when connected
          _sendCommand('S');
        } else {
          _connectionState = 'Connected (Characteristic not found)';
        }
      });

    } catch (e) {
      setState(() {
        _connectionState = 'Connection Failed: $e';
      });
      debugPrint('BLE Connection Error: $e');
    }
  }

  void _sendCommand(String command) async {
    if (_writeCharacteristic != null) {
      try {
        // Automatically check if the characteristic supports writeWithoutResponse
        bool withoutResponse = _writeCharacteristic!.properties.writeWithoutResponse;
        await _writeCharacteristic!.write(utf8.encode(command), withoutResponse: withoutResponse);
        debugPrint('Sent: $command (withoutResponse: $withoutResponse)');
      } catch (e) {
        debugPrint('Write Error: $e');
      }
    }
  }

  String _getCommand(double x, double y) {
    // If stick is near the center, send Stop command ('S')
    if (x.abs() < 0.3 && y.abs() < 0.3) {
      return 'S';
    }

    // Determine direction based on which axis is moved more
    if (y.abs() > x.abs()) {
      // Vertical movement
      return y < 0 ? 'F' : 'B';
    } else {
      // Horizontal movement
      return x < 0 ? 'Q' : 'E';
    }
  }

  void _onJoystickChanged(StickDragDetails details) {
    String command = _getCommand(details.x, details.y);
    
    // Only send if the command changed
    if (command != _lastCommand) {
      setState(() {
        _lastCommand = command;
      });
      _sendCommand(command);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Smart Brick Controller'),
        backgroundColor: Theme.of(context).colorScheme.inversePrimary,
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: [
            Text(
              'Status: $_connectionState',
              textAlign: TextAlign.center,
              style: const TextStyle(fontSize: 18, fontWeight: FontWeight.bold),
            ),
            const SizedBox(height: 20),
            
            // Display currently sent command
            if (_lastCommand != 'S')
              Text('Sending: $_lastCommand', style: const TextStyle(fontSize: 24, color: Colors.blue, fontWeight: FontWeight.bold)),
            if (_lastCommand == 'S')
              const Text('Stopped (Sending: S)', style: const TextStyle(fontSize: 24, color: Colors.grey, fontWeight: FontWeight.bold)),
            
            const SizedBox(height: 80),
            
            // Joystick widget
            Joystick(
              listener: _onJoystickChanged,
            ),
            TextFormField(
              controller: myCustomCommandController,
              decoration: const InputDecoration(
                border: UnderlineInputBorder(),
                labelText: 'Enter command',
              ),
            ),
            TextButton(onPressed: (){_sendCommand(myCustomCommandController.text);}, child: Text('Send custom command'))
          ],
        ),
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: _connectToDevice,
        tooltip: 'Reconnect',
        child: const Icon(Icons.bluetooth),
      ),
    );
  }

  @override
  void dispose() {
    _device?.disconnect();
    super.dispose();
  }
}
