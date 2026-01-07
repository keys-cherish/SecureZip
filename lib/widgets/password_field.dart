import 'package:flutter/material.dart';

/// 密码输入框组件
/// 带有显示/隐藏切换和可选的生成随机密码功能
class PasswordField extends StatefulWidget {
  final TextEditingController controller;
  final String labelText;
  final IconData? prefixIcon;
  final bool showGenerateButton;
  final VoidCallback? onGenerate;
  final String? Function(String?)? validator;

  const PasswordField({
    super.key,
    required this.controller,
    this.labelText = '密码',
    this.prefixIcon,
    this.showGenerateButton = false,
    this.onGenerate,
    this.validator,
  });

  @override
  State<PasswordField> createState() => _PasswordFieldState();
}

class _PasswordFieldState extends State<PasswordField> {
  bool _obscureText = true;

  @override
  Widget build(BuildContext context) {
    return TextFormField(
      controller: widget.controller,
      obscureText: _obscureText,
      decoration: InputDecoration(
        labelText: widget.labelText,
        prefixIcon: widget.prefixIcon != null ? Icon(widget.prefixIcon) : null,
        suffixIcon: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            IconButton(
              icon:
                  Icon(_obscureText ? Icons.visibility : Icons.visibility_off),
              onPressed: () {
                setState(() {
                  _obscureText = !_obscureText;
                });
              },
              tooltip: _obscureText ? '显示密码' : '隐藏密码',
            ),
            if (widget.showGenerateButton && widget.onGenerate != null)
              IconButton(
                icon: const Icon(Icons.casino),
                onPressed: widget.onGenerate,
                tooltip: '生成随机密码',
              ),
          ],
        ),
      ),
      validator: widget.validator,
    );
  }
}
